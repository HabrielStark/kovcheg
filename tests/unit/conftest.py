import asyncio
import pytest


@pytest.fixture
def asyncio_executor():
    """Run async coroutine in event loop within sync test."""

    def _run(coro_or_func, *args, **kwargs):
        if asyncio.iscoroutinefunction(coro_or_func):
            return asyncio.run(coro_or_func(*args, **kwargs))
        elif asyncio.iscoroutine(coro_or_func):
            return asyncio.run(coro_or_func)
        else:
            # wrap sync function into coroutine
            async def _wrapper():
                return coro_or_func(*args, **kwargs)

            return asyncio.run(_wrapper())

    return _run 