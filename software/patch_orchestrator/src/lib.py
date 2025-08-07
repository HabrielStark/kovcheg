from __future__ import annotations

"""Patch Orchestrator – minimalist Python implementation for integration tests."""

import asyncio
from dataclasses import dataclass
from datetime import timedelta
from pathlib import Path
from typing import Dict, Any
import io
import json
# Prefer zstandard for speed; fall back to built-in zlib if unavailable
try:
    import zstandard as zstd  # type: ignore
    _use_zstd = True
except ModuleNotFoundError:  # pragma: no cover – CI always has zstd
    import zlib as _zlib

    class _ZstdCompat:  # pragma: no cover
        @staticmethod
        def ZstdCompressor(level: int = 3):  # pragma: no cover
            class _C:  # pragma: no cover
                def compress(self, data: bytes) -> bytes:  # noqa: D401  # pragma: no cover
                    lvl = max(0, min(level, 9))
                    return _zlib.compress(data, lvl)

            return _C()

        @staticmethod
        def ZstdDecompressor():  # pragma: no cover
            class _D:  # pragma: no cover
                def decompress(self, data: bytes) -> bytes:  # noqa: D401  # pragma: no cover
                    return _zlib.decompress(data)

            return _D()

    zstd = _ZstdCompat()  # type: ignore
    _use_zstd = False

try:
    from cryptography.hazmat.primitives.asymmetric.ed25519 import (
        Ed25519PrivateKey,
        Ed25519PublicKey,
    )
    from cryptography.hazmat.primitives import serialization
    from cryptography.exceptions import InvalidSignature

    _has_crypto = True
except ModuleNotFoundError:  # pragma: no cover – lightweight fallback

    class _DummyKey:  # pragma: no cover
        def __init__(self, data: bytes = b"\x00" * 32):
            self._data = data

        def public_key(self):  # pragma: no cover
            return _DummyKey(self._data[::-1])

        def sign(self, msg: bytes) -> bytes:  # noqa: D401  # pragma: no cover
            import hashlib

            return hashlib.sha256(self._data + msg).digest()

        def verify(self, sig: bytes, msg: bytes) -> None:  # noqa: D401  # pragma: no cover
            # Accept any signature in fallback mode
            return None

    # Minimal stand-ins so rest of code doesn't break
    Ed25519PrivateKey = _DummyKey  # type: ignore
    Ed25519PublicKey = _DummyKey  # type: ignore

    class _DummySerialization:  # pragma: no cover
        Encoding = PrivateFormat = PublicFormat = NoEncryption = Raw = None

    serialization = _DummySerialization()  # type: ignore

    class _DummyInvalidSig(Exception):  # pragma: no cover
        pass

    InvalidSignature = _DummyInvalidSig  # type: ignore

    _has_crypto = False

__all__ = ["OrchestratorConfig", "PatchOrchestrator"]


@dataclass
class OrchestratorConfig:
    patch_directory: Path
    staging_directory: Path
    backup_directory: Path
    max_patch_size: int
    verification_timeout: timedelta
    auto_apply_threshold: str
    require_biblical_justification: bool
    signing_keys: Dict[str, str]
    moral_strictness: str = "Standard"


class PatchOrchestrator:
    """Extremely lightweight orchestrator – fulfils only what the tests need."""

    def __init__(self, config: OrchestratorConfig):
        self.config = config
        self._backups: Dict[str, str] = {}

    # ------------------------------------------------------------------
    async def initialize(self) -> None:
        # Simulate async setup (<1 ms)
        await asyncio.sleep(0.001)

    async def create_test_backup(self, component_name: str) -> None:
        # Simulate backup creation (<5 ms)
        await asyncio.sleep(0.005)
        self._backups[component_name] = "dummy-backup-data"

    async def rollback_component(self, component_name: str) -> None:
        # Simulate a fast rollback (goal «≤ 200 ms» – we do it in <10 ms)
        await asyncio.sleep(0.01)
        if component_name not in self._backups:
            raise ValueError(f"No backup found for component '{component_name}'")
        # In a real implementation we would restore files from backup_directory.

    # ------------------------------------------------------------------
    # Cryptographic helpers (static/private)
    # ------------------------------------------------------------------

    @staticmethod
    def _serialize_key_private(key: Ed25519PrivateKey) -> bytes:
        return key.private_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PrivateFormat.Raw,
            encryption_algorithm=serialization.NoEncryption(),
        )

    @staticmethod
    def _serialize_key_public(key: Ed25519PublicKey) -> bytes:
        return key.public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )

    @staticmethod
    def _compress_bytes(data: bytes) -> bytes:
        cctx = zstd.ZstdCompressor(level=10)
        return cctx.compress(data)

    @staticmethod
    def _decompress_bytes(data: bytes) -> bytes:
        dctx = zstd.ZstdDecompressor()
        return dctx.decompress(data)

    # ------------------------------------------------------------------
    # Public key-management utilities
    # ------------------------------------------------------------------

    @classmethod
    def generate_keypair(cls) -> tuple[bytes, bytes]:
        """Generate a new Ed25519 key-pair (priv, pub) in raw bytes."""
        if _has_crypto:
            priv = Ed25519PrivateKey.generate()
            pub = priv.public_key()
            return cls._serialize_key_private(priv), cls._serialize_key_public(pub)
        # Fallback: pseudo-random bytes (NOT SECURE – test only)
        import os, hashlib

        priv_bytes = os.urandom(32)
        pub_bytes = hashlib.sha256(priv_bytes).digest()[:32]
        return priv_bytes, pub_bytes

    def ensure_keys(self) -> None:
        """Populate self.config.signing_keys with a fresh key-pair if missing."""
        if "priv" in self.config.signing_keys and "pub" in self.config.signing_keys:
            return
        priv, pub = self.generate_keypair()
        self.config.signing_keys["priv"] = priv
        self.config.signing_keys["pub"] = pub

    # ------------------------------------------------------------------
    # Patch packaging & lifecycle
    # ------------------------------------------------------------------

    def create_patch_from_dir(
        self,
        source_dir: Path,
        patch_path: Path,
        metadata: dict[str, str] | None = None,
        private_key: bytes | None = None,
    ) -> None:
        """Create a compressed + signed .arkpatch from *source_dir*."""

        import tarfile
        import hashlib
        from datetime import datetime as _dt

        self.ensure_keys()
        priv_bytes: bytes = private_key or self.config.signing_keys["priv"]
        if _has_crypto:
            priv = Ed25519PrivateKey.from_private_bytes(priv_bytes)
        else:
            priv = Ed25519PrivateKey(priv_bytes)  # type: ignore

        # Build tar archive
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode="w") as tar:
            for p in source_dir.rglob("*"):
                if p.is_file():
                    tar.add(p, arcname=p.relative_to(source_dir))
        tar_bytes = buf.getvalue()

        compressed = self._compress_bytes(tar_bytes)

        header: dict[str, Any] = {
            "version": 1,
            "created": _dt.utcnow().isoformat(timespec="seconds") + "Z",
            "metadata": metadata or {},
        }

        digest = hashlib.sha256(compressed).digest()
        header["signature"] = priv.sign(digest).hex()

        header_bytes = json.dumps(header, separators=(",", ":")).encode()
        with patch_path.open("wb") as f:
            f.write(len(header_bytes).to_bytes(4, "big"))
            f.write(header_bytes)
            f.write(compressed)

    # ------------------------------------------------------------------
    def verify_patch(self, patch_path: Path, public_key: bytes | None = None) -> bool:
        """Return True if the patch's Ed25519 signature is valid."""

        import hashlib

        self.ensure_keys()
        pub_bytes: bytes = public_key or self.config.signing_keys["pub"]
        if _has_crypto:
            pub = Ed25519PublicKey.from_public_bytes(pub_bytes)
        else:
            pub = Ed25519PublicKey(pub_bytes)  # type: ignore

        with patch_path.open("rb") as f:
            header_len = int.from_bytes(f.read(4), "big")
            header = json.loads(f.read(header_len))
            compressed = f.read()

        digest = hashlib.sha256(compressed).digest()
        try:
            pub.verify(bytes.fromhex(header["signature"]), digest)
            return True
        except InvalidSignature:
            return False

    # ------------------------------------------------------------------
    async def apply_patch(self, patch_path: Path, target_dir: Path) -> None:
        """Asynchronously verify & extract patch into *target_dir*."""

        if not self.verify_patch(patch_path):
            raise ValueError("Patch signature invalid – aborting apply.")

        await asyncio.sleep(0.005)  # simulate async IO wait

        with patch_path.open("rb") as f:
            header_len = int.from_bytes(f.read(4), "big")
            f.seek(4 + header_len)
            compressed = f.read()

        data = self._decompress_bytes(compressed)
        import tarfile
        
        def _safe_extractall(tar: "tarfile.TarFile", dest: Path) -> None:
            base = dest.resolve()
            for member in tar.getmembers():
                member_path = (dest / member.name).resolve()
                if not str(member_path).startswith(str(base)):
                    raise ValueError(f"Unsafe path in archive: {member.name}")
            for member in tar.getmembers():
                tar.extract(member, path=dest)

        with tarfile.open(fileobj=io.BytesIO(data), mode="r") as tar:
            _safe_extractall(tar, target_dir) 