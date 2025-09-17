// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ArkBounty - Role aware bug bounty escrow with DAO governance hooks
/// @notice Provides configurable bounty programmes with strict access control,
///         payout delays and maintainer limits. Designed to replace the earlier
///         proof-of-concept contract that only handled a happy path.
contract ArkBounty {
    enum Severity {
        Info,
        Low,
        Medium,
        High,
        Critical
    }

    enum SubmissionStatus {
        Submitted,
        InReview,
        Approved,
        Rejected,
        Paid
    }

    struct Program {
        bool exists;
        bool active;
        uint256 totalBudget;
        uint256 lockedBudget;
        uint256 maxReward;
        uint256 maintainerLimit;
        uint64 createdAt;
        uint64 updatedAt;
    }

    struct Submission {
        address hunter;
        string uri;
        Severity severity;
        SubmissionStatus status;
        uint256 reward;
        uint64 submittedAt;
        uint64 decidedAt;
    }

    event ProgramConfigured(bytes32 indexed programId, bool active, uint256 maxReward, uint256 maintainerLimit);
    event ProgramFunded(bytes32 indexed programId, address indexed funder, uint256 amount, uint256 newBudget);
    event MaintainerUpdated(address indexed account, bool enabled);
    event FindingSubmitted(bytes32 indexed programId, uint256 indexed submissionId, address indexed hunter, Severity severity, string uri);
    event SubmissionReviewed(bytes32 indexed programId, uint256 indexed submissionId, SubmissionStatus status, uint256 reward);
    event RewardClaimed(bytes32 indexed programId, uint256 indexed submissionId, address indexed hunter, uint256 value);
    event TreasuryUpdated(address indexed newTreasury);
    event PayoutDelayUpdated(uint256 newDelaySeconds);

    address public owner;
    address public daoTreasury;
    uint256 public payoutDelay = 1 days;

    mapping(address => bool) public maintainers;
    mapping(bytes32 => Program) private programs;
    mapping(bytes32 => uint256) private submissionCounters;
    mapping(bytes32 => mapping(uint256 => Submission)) private submissions;

    bool private _reentrancyGuard;

    modifier onlyOwner() {
        require(msg.sender == owner, "ARK:NOT_OWNER");
        _;
    }

    modifier onlyMaintainerOrGovernor() {
        require(maintainers[msg.sender] || _isGovernor(msg.sender), "ARK:NOT_AUTHORISED");
        _;
    }

    modifier onlyGovernor() {
        require(_isGovernor(msg.sender), "ARK:NOT_GOVERNOR");
        _;
    }

    modifier nonReentrant() {
        require(!_reentrancyGuard, "ARK:REENTRANT");
        _reentrancyGuard = true;
        _;
        _reentrancyGuard = false;
    }

    constructor(address initialTreasury) {
        owner = msg.sender;
        daoTreasury = initialTreasury;
    }

    // ---------------------------------------------------------------------
    // Programme lifecycle
    // ---------------------------------------------------------------------

    function configureProgram(
        bytes32 programId,
        bool active,
        uint256 maxReward,
        uint256 maintainerLimit
    ) external onlyGovernor {
        require(maxReward > 0, "ARK:MAX_REWARD_ZERO");
        require(maintainerLimit <= maxReward, "ARK:LIMIT_GT_MAX");

        Program storage program = programs[programId];
        if (!program.exists) {
            program.exists = true;
            program.createdAt = uint64(block.timestamp);
        }

        program.active = active;
        program.maxReward = maxReward;
        program.maintainerLimit = maintainerLimit;
        program.updatedAt = uint64(block.timestamp);

        emit ProgramConfigured(programId, active, maxReward, maintainerLimit);
    }

    function fundProgram(bytes32 programId) external payable {
        Program storage program = programs[programId];
        require(program.exists, "ARK:UNKNOWN_PROGRAM");
        require(msg.value > 0, "ARK:FUND_ZERO");

        program.totalBudget += msg.value;
        emit ProgramFunded(programId, msg.sender, msg.value, program.totalBudget);
    }

    function setPayoutDelay(uint256 newDelay) external onlyGovernor {
        require(newDelay <= 30 days, "ARK:DELAY_TOO_LARGE");
        payoutDelay = newDelay;
        emit PayoutDelayUpdated(newDelay);
    }

    function setDaoTreasury(address newTreasury) external onlyOwner {
        daoTreasury = newTreasury;
        emit TreasuryUpdated(newTreasury);
    }

    function setMaintainer(address account, bool enabled) external onlyGovernor {
        maintainers[account] = enabled;
        emit MaintainerUpdated(account, enabled);
    }

    // ---------------------------------------------------------------------
    // Submission handling
    // ---------------------------------------------------------------------

    function submitFinding(
        bytes32 programId,
        string calldata uri,
        Severity severity
    ) external returns (uint256 submissionId) {
        Program storage program = programs[programId];
        require(program.exists, "ARK:UNKNOWN_PROGRAM");
        require(program.active, "ARK:PROGRAM_PAUSED");

        submissionId = ++submissionCounters[programId];
        Submission storage submission = submissions[programId][submissionId];
        submission.hunter = msg.sender;
        submission.uri = uri;
        submission.severity = severity;
        submission.status = SubmissionStatus.Submitted;
        submission.submittedAt = uint64(block.timestamp);

        emit FindingSubmitted(programId, submissionId, msg.sender, severity, uri);
    }

    function markInReview(bytes32 programId, uint256 submissionId) external onlyMaintainerOrGovernor {
        Submission storage submission = _requireSubmission(programId, submissionId);
        require(
            submission.status == SubmissionStatus.Submitted,
            "ARK:STATUS_FINAL"
        );
        submission.status = SubmissionStatus.InReview;
        emit SubmissionReviewed(programId, submissionId, SubmissionStatus.InReview, 0);
    }

    function reviewSubmission(
        bytes32 programId,
        uint256 submissionId,
        bool approve,
        uint256 reward
    ) external onlyMaintainerOrGovernor {
        Program storage program = programs[programId];
        require(program.exists, "ARK:UNKNOWN_PROGRAM");

        Submission storage submission = _requireSubmission(programId, submissionId);
        require(
            submission.status == SubmissionStatus.Submitted ||
                submission.status == SubmissionStatus.InReview,
            "ARK:STATUS_FINAL"
        );

        if (approve) {
            require(reward > 0, "ARK:REWARD_ZERO");
            require(reward <= program.maxReward, "ARK:REWARD_GT_MAX");
            if (!_isGovernor(msg.sender)) {
                require(maintainers[msg.sender], "ARK:NOT_MAINTAINER");
                require(reward <= program.maintainerLimit, "ARK:ABOVE_LIMIT");
            }
            require(program.totalBudget >= program.lockedBudget + reward, "ARK:BUDGET_LOW");

            submission.reward = reward;
            submission.status = SubmissionStatus.Approved;
            submission.decidedAt = uint64(block.timestamp);
            program.lockedBudget += reward;
        } else {
            submission.reward = 0;
            submission.status = SubmissionStatus.Rejected;
            submission.decidedAt = uint64(block.timestamp);
        }

        emit SubmissionReviewed(programId, submissionId, submission.status, submission.reward);
    }

    function claimReward(bytes32 programId, uint256 submissionId) external nonReentrant {
        Program storage program = programs[programId];
        require(program.exists, "ARK:UNKNOWN_PROGRAM");

        Submission storage submission = _requireSubmission(programId, submissionId);
        require(submission.status == SubmissionStatus.Approved, "ARK:NOT_APPROVED");
        require(msg.sender == submission.hunter, "ARK:NOT_HUNTER");
        require(block.timestamp >= submission.decidedAt + payoutDelay, "ARK:DELAY_ACTIVE");

        uint256 reward = submission.reward;
        require(reward > 0, "ARK:NO_REWARD");
        submission.status = SubmissionStatus.Paid;
        submission.reward = 0;
        program.lockedBudget -= reward;
        program.totalBudget -= reward;

        (bool sent, ) = payable(msg.sender).call{value: reward}("");
        require(sent, "ARK:TRANSFER_FAIL");

        emit RewardClaimed(programId, submissionId, msg.sender, reward);
    }

    function withdrawExcess(
        bytes32 programId,
        uint256 amount,
        address payable recipient
    ) external onlyGovernor nonReentrant {
        Program storage program = programs[programId];
        require(program.exists, "ARK:UNKNOWN_PROGRAM");
        require(amount > 0, "ARK:AMOUNT_ZERO");
        require(recipient != address(0), "ARK:BAD_RECIPIENT");
        require(program.totalBudget >= program.lockedBudget + amount, "ARK:BUDGET_LOW");

        program.totalBudget -= amount;
        (bool ok, ) = recipient.call{value: amount}("");
        require(ok, "ARK:TRANSFER_FAIL");
    }

    // ---------------------------------------------------------------------
    // Views
    // ---------------------------------------------------------------------

    function getProgram(bytes32 programId) external view returns (Program memory) {
        return programs[programId];
    }

    function getSubmission(bytes32 programId, uint256 submissionId) external view returns (Submission memory) {
        return submissions[programId][submissionId];
    }

    function nextSubmissionId(bytes32 programId) external view returns (uint256) {
        return submissionCounters[programId] + 1;
    }

    // ---------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------

    function _isGovernor(address account) internal view returns (bool) {
        if (account == owner) {
            return true;
        }
        return daoTreasury != address(0) && account == daoTreasury;
    }

    function _requireSubmission(bytes32 programId, uint256 submissionId)
        internal
        view
        returns (Submission storage)
    {
        Submission storage submission = submissions[programId][submissionId];
        require(submission.hunter != address(0), "ARK:UNKNOWN_SUBMISSION");
        return submission;
    }
}
