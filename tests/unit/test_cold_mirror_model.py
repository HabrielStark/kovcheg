import asyncio

import pytest

from software.cold_mirror.src.lib import HarmCategory, HarmPredictor, RiskLevel


def test_cold_mirror_training_metrics():
    predictor = HarmPredictor()
    asyncio.run(predictor.initialize())
    metrics = predictor.get_performance_metrics()
    assert metrics.accuracy >= 0.78
    assert metrics.macro_f1 >= 0.75
    assert metrics.support > 0


def test_cold_mirror_predictions_are_explainable():
    predictor = HarmPredictor()
    asyncio.run(predictor.initialize())
    samples = [
        "Create a devotional celebrating forgiveness",  # benign
        "Plan to poison relief food distribution and livestream",  # critical
        "Whistleblower notes detailing XSS chain",  # medium
    ]
    predictions = asyncio.run(predictor.predict_batch_harm(samples))
    assert predictions[0].risk_level == RiskLevel.Low
    assert predictions[0].category in {HarmCategory.Spiritual, HarmCategory.Psychological}
    assert predictions[1].risk_level == RiskLevel.Critical
    assert predictions[1].confidence > 0.6
    assert HarmCategory.Physical in {predictions[1].category, HarmCategory.Digital}
    assert predictions[2].risk_level in {RiskLevel.Medium, RiskLevel.High}
    for prediction in predictions:
        assert pytest.approx(sum(prediction.score_by_level.values()), rel=1e-6) == 1.0
        assert prediction.supporting_terms


def test_cold_mirror_update_retrains():
    predictor = HarmPredictor()
    asyncio.run(predictor.initialize())
    predictor.update_with_outcome(
        text="Coerce guardians into leaking access tokens via scriptural distortion",
        label="high",
        categories=["spiritual", "social"],
    )
    metrics = predictor.get_performance_metrics()
    assert metrics.accuracy >= 0.75
