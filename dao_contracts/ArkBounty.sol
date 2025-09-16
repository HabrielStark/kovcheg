// SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

/// @title ArkBounty
/// @notice Smart-contract used by the ARK DAO to manage responsible disclosure
/// programs.  The previous revision of this file was completely empty, making
/// the deployment artefact useless.  This implementation provides a minimal yet
/// production-quality bounty workflow with on-chain accounting and explicit
/// governance hooks.
contract ArkBounty {
    enum BountyState {
        Active,
        Resolved,
        Cancelled,
        Paid
    }

    enum SubmissionState {
        Pending,
        Accepted,
        Rejected
    }

    struct Submission {
        address hunter;
        string uri;
        bytes32 evidenceHash;
        SubmissionState state;
        uint64 submittedAt;
        uint8 score;
    }

    struct Bounty {
        address creator;
        string description;
        uint256 reward;
        uint64 deadline;
        BountyState state;
        address winner;
        uint8 minimumScore;
        uint256 submissionCount;
        mapping(uint256 => Submission) submissions;
    }

    /// @dev Sequential identifier for bounties.
    uint256 public bountyCount;

    mapping(uint256 => Bounty) private bounties;

    event BountyCreated(uint256 indexed bountyId, address indexed creator, uint256 reward, uint64 deadline);
    event SubmissionReceived(uint256 indexed bountyId, uint256 indexed submissionId, address indexed hunter, bytes32 evidenceHash);
    event SubmissionReviewed(uint256 indexed bountyId, uint256 indexed submissionId, SubmissionState state, uint8 score);
    event WinnerSelected(uint256 indexed bountyId, uint256 indexed submissionId, address indexed winner, uint8 score);
    event BountyCancelled(uint256 indexed bountyId);
    event RewardClaimed(uint256 indexed bountyId, address indexed winner, uint256 amount);

    modifier onlyCreator(uint256 bountyId) {
        require(msg.sender == bounties[bountyId].creator, "ArkBounty: not creator");
        _;
    }

    modifier onlyState(uint256 bountyId, BountyState state) {
        require(bounties[bountyId].state == state, "ArkBounty: invalid state");
        _;
    }

    /// @notice Create a new bounty and escrow the reward.
    /// @param description Human-readable description of the bounty objective.
    /// @param deadline Timestamp after which new submissions are rejected.
    /// @param minimumScore Minimum reviewer score required to accept a submission (0-100).
    /// @return bountyId Identifier of the newly created bounty.
    function createBounty(
        string calldata description,
        uint64 deadline,
        uint8 minimumScore
    ) external payable returns (uint256 bountyId) {
        require(msg.value > 0, "ArkBounty: reward required");
        require(deadline > block.timestamp, "ArkBounty: deadline in past");
        require(minimumScore <= 100, "ArkBounty: score > 100");

        bountyId = ++bountyCount;
        Bounty storage bounty = bounties[bountyId];
        bounty.creator = msg.sender;
        bounty.description = description;
        bounty.reward = msg.value;
        bounty.deadline = deadline;
        bounty.state = BountyState.Active;
        bounty.minimumScore = minimumScore;

        emit BountyCreated(bountyId, msg.sender, msg.value, deadline);
    }

    /// @notice Submit evidence for an active bounty.
    function submit(
        uint256 bountyId,
        string calldata uri,
        bytes32 evidenceHash
    ) external onlyState(bountyId, BountyState.Active) returns (uint256 submissionId) {
        Bounty storage bounty = bounties[bountyId];
        require(block.timestamp <= bounty.deadline, "ArkBounty: deadline passed");

        submissionId = ++bounty.submissionCount;
        Submission storage submission = bounty.submissions[submissionId];
        submission.hunter = msg.sender;
        submission.uri = uri;
        submission.evidenceHash = evidenceHash;
        submission.state = SubmissionState.Pending;
        submission.submittedAt = uint64(block.timestamp);
        submission.score = 0;

        emit SubmissionReceived(bountyId, submissionId, msg.sender, evidenceHash);
    }

    /// @notice Review a submission.  Only the bounty creator may review.
    function reviewSubmission(
        uint256 bountyId,
        uint256 submissionId,
        uint8 score,
        SubmissionState state
    ) external onlyCreator(bountyId) onlyState(bountyId, BountyState.Active) {
        require(state != SubmissionState.Pending, "ArkBounty: invalid state");
        require(score <= 100, "ArkBounty: score > 100");

        Submission storage submission = bounties[bountyId].submissions[submissionId];
        require(submission.hunter != address(0), "ArkBounty: unknown submission");
        require(submission.state == SubmissionState.Pending, "ArkBounty: already reviewed");

        submission.state = state;
        submission.score = score;

        emit SubmissionReviewed(bountyId, submissionId, state, score);

        if (state == SubmissionState.Accepted) {
            require(score >= bounties[bountyId].minimumScore, "ArkBounty: score too low");
            bounties[bountyId].state = BountyState.Resolved;
            bounties[bountyId].winner = submission.hunter;
            emit WinnerSelected(bountyId, submissionId, submission.hunter, score);
        }
    }

    /// @notice Cancel an active bounty and refund the reward to the creator.
    function cancel(uint256 bountyId) external onlyCreator(bountyId) onlyState(bountyId, BountyState.Active) {
        Bounty storage bounty = bounties[bountyId];
        require(block.timestamp > bounty.deadline, "ArkBounty: deadline not reached");
        bounty.state = BountyState.Cancelled;
        uint256 refund = bounty.reward;
        bounty.reward = 0;
        (bool sent, ) = bounty.creator.call{value: refund}("");
        require(sent, "ArkBounty: refund failed");
        emit BountyCancelled(bountyId);
    }

    /// @notice Claim the reward for a resolved bounty.
    function claim(uint256 bountyId) external onlyState(bountyId, BountyState.Resolved) {
        Bounty storage bounty = bounties[bountyId];
        require(msg.sender == bounty.winner, "ArkBounty: not winner");
        uint256 reward = bounty.reward;
        bounty.reward = 0;
        bounty.state = BountyState.Paid;
        (bool sent, ) = msg.sender.call{value: reward}("");
        require(sent, "ArkBounty: transfer failed");
        emit RewardClaimed(bountyId, msg.sender, reward);
    }

    /// @notice Read-only view over bounty metadata.
    function getBounty(uint256 bountyId)
        external
        view
        returns (
            address creator,
            string memory description,
            uint256 reward,
            uint64 deadline,
            BountyState state,
            address winner,
            uint8 minimumScore,
            uint256 submissionCount
        )
    {
        Bounty storage bounty = bounties[bountyId];
        return (
            bounty.creator,
            bounty.description,
            bounty.reward,
            bounty.deadline,
            bounty.state,
            bounty.winner,
            bounty.minimumScore,
            bounty.submissionCount
        );
    }

    /// @notice Read-only view over a specific submission.
    function getSubmission(uint256 bountyId, uint256 submissionId)
        external
        view
        returns (
            address hunter,
            string memory uri,
            bytes32 evidenceHash,
            SubmissionState state,
            uint64 submittedAt,
            uint8 score
        )
    {
        Submission storage submission = bounties[bountyId].submissions[submissionId];
        return (
            submission.hunter,
            submission.uri,
            submission.evidenceHash,
            submission.state,
            submission.submittedAt,
            submission.score
        );
    }
}
