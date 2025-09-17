// SPDX-License-Identifier: CC0-1.0
pragma solidity ^0.8.20;

/// @title ArkBounty - structured bounty programme with access control
contract ArkBounty {
    enum BountyState { Draft, Active, Submitted, Approved, Paid, Cancelled }

    struct Bounty {
        address creator;
        string description;
        uint256 reward;
        uint256 deadline;
        BountyState state;
        address hunter;
        uint256 submissionId;
    }

    struct Submission {
        address hunter;
        string uri;
        uint256 submittedAt;
        uint8 score;
        bool approved;
    }

    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant REVIEWER_ROLE = keccak256("REVIEWER_ROLE");
    bytes32 public constant TREASURER_ROLE = keccak256("TREASURER_ROLE");

    mapping(address => mapping(bytes32 => bool)) private _roles;
    mapping(uint256 => Bounty) public bounties;
    mapping(uint256 => Submission) public submissions;
    mapping(address => uint256) public activeAssignments;

    uint256 public nextBountyId = 1;
    uint256 public nextSubmissionId = 1;
    uint256 public treasuryBalance;
    uint256 public lockedRewards;
    uint256 public maxRewardPerBounty = 100 ether;
    uint256 public maxActivePerHunter = 3;

    bool private _entered;

    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);
    event BountyCreated(uint256 indexed bountyId, address indexed creator, uint256 reward, uint256 deadline);
    event SubmissionReceived(uint256 indexed bountyId, uint256 indexed submissionId, address indexed hunter);
    event SubmissionReviewed(uint256 indexed bountyId, uint256 indexed submissionId, bool approved, uint8 score);
    event BountyPaid(uint256 indexed bountyId, address indexed hunter, uint256 amount);
    event TreasuryDeposited(address indexed from, uint256 amount);
    event TreasuryWithdrawn(address indexed to, uint256 amount);

    modifier onlyRole(bytes32 role) {
        require(_roles[msg.sender][role], "ARK: missing role");
        _;
    }

    modifier nonReentrant() {
        require(!_entered, "ARK: reentrant call");
        _entered = true;
        _;
        _entered = false;
    }

    constructor() {
        _roles[msg.sender][ADMIN_ROLE] = true;
        _roles[msg.sender][TREASURER_ROLE] = true;
        emit RoleGranted(ADMIN_ROLE, msg.sender, msg.sender);
        emit RoleGranted(TREASURER_ROLE, msg.sender, msg.sender);
    }

    // ------------------------------------------------------------------
    function hasRole(bytes32 role, address account) public view returns (bool) {
        return _roles[account][role];
    }

    function grantRole(bytes32 role, address account) external onlyRole(ADMIN_ROLE) {
        _roles[account][role] = true;
        emit RoleGranted(role, account, msg.sender);
    }

    function revokeRole(bytes32 role, address account) external onlyRole(ADMIN_ROLE) {
        _roles[account][role] = false;
        emit RoleRevoked(role, account, msg.sender);
    }

    // ------------------------------------------------------------------
    function setMaxReward(uint256 newMax) external onlyRole(ADMIN_ROLE) {
        require(newMax > 0, "ARK: max reward must be positive");
        maxRewardPerBounty = newMax;
    }

    function setMaxActivePerHunter(uint256 newLimit) external onlyRole(ADMIN_ROLE) {
        require(newLimit > 0, "ARK: limit must be positive");
        maxActivePerHunter = newLimit;
    }

    // ------------------------------------------------------------------
    function deposit() external payable onlyRole(TREASURER_ROLE) {
        treasuryBalance += msg.value;
        emit TreasuryDeposited(msg.sender, msg.value);
    }

    function emergencyWithdraw(address payable to, uint256 amount) external onlyRole(TREASURER_ROLE) {
        require(to != address(0), "ARK: invalid recipient");
        require(amount <= treasuryBalance - lockedRewards, "ARK: funds locked");
        treasuryBalance -= amount;
        (bool sent, ) = to.call{value: amount}("");
        require(sent, "ARK: withdrawal failed");
        emit TreasuryWithdrawn(to, amount);
    }

    // ------------------------------------------------------------------
    function createBounty(string calldata description, uint256 reward, uint256 deadline) external onlyRole(ADMIN_ROLE) returns (uint256) {
        require(bytes(description).length > 0, "ARK: description required");
        require(reward > 0 && reward <= maxRewardPerBounty, "ARK: invalid reward");
        require(deadline > block.timestamp, "ARK: deadline in the past");
        require(treasuryBalance >= lockedRewards + reward, "ARK: insufficient treasury");

        uint256 bountyId = nextBountyId++;
        bounties[bountyId] = Bounty({
            creator: msg.sender,
            description: description,
            reward: reward,
            deadline: deadline,
            state: BountyState.Active,
            hunter: address(0),
            submissionId: 0
        });
        lockedRewards += reward;
        emit BountyCreated(bountyId, msg.sender, reward, deadline);
        return bountyId;
    }

    function cancelBounty(uint256 bountyId) external onlyRole(ADMIN_ROLE) {
        Bounty storage bounty = bounties[bountyId];
        require(bounty.state == BountyState.Active, "ARK: cannot cancel");
        bounty.state = BountyState.Cancelled;
        lockedRewards -= bounty.reward;
    }

    function submitFinding(uint256 bountyId, string calldata uri) external returns (uint256) {
        Bounty storage bounty = bounties[bountyId];
        require(bounty.state == BountyState.Active, "ARK: not accepting submissions");
        require(block.timestamp <= bounty.deadline, "ARK: deadline passed");
        require(activeAssignments[msg.sender] < maxActivePerHunter, "ARK: hunter limit reached");

        uint256 submissionId = nextSubmissionId++;
        submissions[submissionId] = Submission({
            hunter: msg.sender,
            uri: uri,
            submittedAt: block.timestamp,
            score: 0,
            approved: false
        });

        bounty.state = BountyState.Submitted;
        bounty.hunter = msg.sender;
        bounty.submissionId = submissionId;
        activeAssignments[msg.sender] += 1;

        emit SubmissionReceived(bountyId, submissionId, msg.sender);
        return submissionId;
    }

    function reviewSubmission(uint256 bountyId, uint8 score, bool approve) external onlyRole(REVIEWER_ROLE) {
        Bounty storage bounty = bounties[bountyId];
        require(bounty.state == BountyState.Submitted, "ARK: nothing to review");
        Submission storage submission = submissions[bounty.submissionId];
        require(submission.hunter != address(0), "ARK: submission missing");

        submission.score = score;
        submission.approved = approve;

        if (approve) {
            bounty.state = BountyState.Approved;
        } else {
            bounty.state = BountyState.Active;
            activeAssignments[submission.hunter] -= 1;
            bounty.hunter = address(0);
            bounty.submissionId = 0;
        }
        emit SubmissionReviewed(bountyId, bounty.submissionId, approve, score);
    }

    function payout(uint256 bountyId, address payable recipient) external onlyRole(TREASURER_ROLE) nonReentrant {
        Bounty storage bounty = bounties[bountyId];
        require(bounty.state == BountyState.Approved, "ARK: bounty not approved");
        require(recipient != address(0), "ARK: invalid recipient");

        Submission storage submission = submissions[bounty.submissionId];
        require(submission.approved, "ARK: submission not approved");

        bounty.state = BountyState.Paid;
        activeAssignments[submission.hunter] = activeAssignments[submission.hunter] > 0 ? activeAssignments[submission.hunter] - 1 : 0;
        lockedRewards -= bounty.reward;
        treasuryBalance -= bounty.reward;

        (bool sent, ) = recipient.call{value: bounty.reward}("");
        require(sent, "ARK: payout failed");

        emit BountyPaid(bountyId, recipient, bounty.reward);
    }
}
