# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The rendered JVM runtime is byte-identical to the pinned Java and Kotlin oracle SDKs' files.

Oracles are checkouts at ``oracles/cache/<name>`` (pins in ``runtime/README.md``); a test skips
when its checkout is absent. All four are Apache-2.0: the template comes from OneBusAway's
java-sdk and kotlin-sdk, with ``OkHttpClient.kt`` and the webhook files from imagekit-java.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

from ferrule_runtime import JavaOptions, render_java_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"


@dataclass(frozen=True)
class Oracle:
    name: str
    options: JavaOptions
    known_drift: frozenset[str] = frozenset()
    """Files whose text differs for a recorded reason (runtime/README.md), relative to
    ``<artifact>-core/src/main/kotlin/<package>/`` (``test:``: the core module's tests;
    ``okhttp:`` / ``okhttp-test:``: the client-okhttp module) unless they start with the
    artifact."""
    generated: frozenset[str] = frozenset()
    """Runtime-adjacent files the plugin generates (pagination), relative like known_drift."""

    def path(self, rel: str) -> str:
        if rel.startswith(self.options.artifact_name):
            return rel
        package = self.options.package_name.replace(".", "/")
        side, rest = rel.split(":", 1) if ":" in rel else ("main", rel)
        module, side = {
            "okhttp": ("client-okhttp", "main"),
            "okhttp-test": ("client-okhttp", "test"),
        }.get(side, ("core", side))
        return f"{self.options.artifact_name}-{module}/src/{side}/kotlin/{package}/{rest}"


# knock-java's pin is a 2025 generation: these files have older text or are absent (the
# sleeper and logging classes, the retryable exception and the ProGuard rules came later).
_KNOCK_2025 = frozenset(
    {
        "okhttp:client/okhttp/OkHttpClient.kt",
        "okhttp-test:client/okhttp/OkHttpClientTest.kt",
        "knock-java-core/src/main/resources/META-INF/proguard/knock-java-core.pro",
        "core/DefaultSleeper.kt",
        "core/LogLevel.kt",
        "core/ObjectMappers.kt",
        "core/PhantomReachableExecutorService.kt",
        "core/PhantomReachableSleeper.kt",
        "core/Properties.kt",
        "core/RequestOptions.kt",
        "core/Sleeper.kt",
        "core/Timeout.kt",
        "core/Utils.kt",
        "core/handlers/ErrorHandler.kt",
        "core/http/Headers.kt",
        "core/http/HttpRequest.kt",
        "core/http/HttpRequestBodies.kt",
        "core/http/LoggingHttpClient.kt",
        "core/http/ProxyAuthenticator.kt",
        "core/http/QueryParams.kt",
        "core/http/RetryingHttpClient.kt",
        "errors/BadRequestException.kt",
        "errors/InternalServerException.kt",
        "errors/KnockRetryableException.kt",
        "errors/KnockServiceException.kt",
        "errors/NotFoundException.kt",
        "errors/PermissionDeniedException.kt",
        "errors/RateLimitException.kt",
        "errors/UnauthorizedException.kt",
        "errors/UnexpectedStatusCodeException.kt",
        "errors/UnprocessableEntityException.kt",
        "test:TestServerExtension.kt",
        "test:core/ObjectMappersTest.kt",
        "test:core/http/HttpRequestBodiesTest.kt",
        "test:core/http/HttpRequestTest.kt",
        "test:core/http/LoggingHttpClientTest.kt",
        "test:core/http/RetryingHttpClientTest.kt",
    }
)

ORACLES = [
    Oracle(
        "onebusaway-java",
        JavaOptions(
            package_name="org.onebusaway",
            client_name="OnebusawaySdk",
            artifact_name="onebusaway-sdk-java",
            repo="OneBusAway/java-sdk",
            array_format="repeat",
        ),
        # A hand edit: a println `LoggingInterceptor` added below the generated class.
        frozenset({"okhttp:client/okhttp/OkHttpClient.kt"}),
    ),
    Oracle(
        "onebusaway-kotlin",
        JavaOptions(
            package_name="org.onebusaway",
            client_name="OnebusawaySdk",
            artifact_name="onebusaway-sdk-kotlin",
            repo="OneBusAway/kotlin-sdk",
            target="kotlin",
            array_format="repeat",
        ),
    ),
    Oracle(
        "imagekit-java",
        JavaOptions(
            package_name="io.imagekit",
            client_name="ImageKit",
            artifact_name="image-kit-java",
            repo="imagekit-developer/imagekit-java",
            webhooks=True,
            empty_responses=True,
            mock_tests=False,
        ),
    ),
    Oracle(
        "knock-java",
        JavaOptions(
            package_name="app.knock.api",
            client_name="Knock",
            artifact_name="knock-java",
            repo="knocklabs/knock-java",
        ),
        _KNOCK_2025,
        frozenset(
            {
                "core/AutoPager.kt",
                "core/AutoPagerAsync.kt",
                "core/Page.kt",
                "core/PageAsync.kt",
                "test:core/AutoPagerTest.kt",
                "test:core/AutoPagerAsyncTest.kt",
            }
        ),
    ),
]
# Mixed with client settings (auth, environment variables, base URL): generated, like
# TypeScript's src/client.ts.
GENERATED = ("core/ClientOptions.kt", "test:core/ClientOptionsTest.kt")


def _root(oracle: Oracle) -> Path:
    root = CACHE / oracle.name
    if not (root / "settings.gradle.kts").is_file():
        pytest.skip(f"oracle checkout absent: oracles/cache/{oracle.name}")
    return root


def _matches(root: Path, rel: str, text: str) -> bool:
    path = root / rel
    return path.is_file() and path.read_bytes() == text.encode()


def _drift(oracle: Oracle) -> set[str]:
    return {oracle.path(rel) for rel in oracle.known_drift}


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_matches_oracle_bytes(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_java_files(oracle.options)
    drift = _drift(oracle)
    mismatched = sorted(
        rel for rel, text in files.items() if rel not in drift and not _matches(root, rel, text)
    )
    assert mismatched == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_known_drift_is_still_drift(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_java_files(oracle.options)
    stale = sorted(rel for rel in _drift(oracle) if _matches(root, rel, files[rel]))
    assert stale == []


def _on_disk(root: Path, oracle: Oracle) -> set[str]:
    artifact = oracle.options.artifact_name
    package = oracle.options.package_name.replace(".", "/")
    core = f"{artifact}-core/src"
    tops = [
        f"{core}/main/kotlin/{package}/core",
        f"{core}/main/kotlin/{package}/errors",
        f"{core}/test/kotlin/{package}/core",
        f"{core}/main/resources/META-INF/proguard",
    ]
    found = {
        p.relative_to(root).as_posix()
        for top in tops
        for p in (root / top).rglob("*")
        if p.is_file()
    }
    found |= {
        p.relative_to(root).as_posix()
        for side in ("main", "test")
        for p in (root / f"{artifact}-client-okhttp/src/{side}/kotlin/{package}").rglob(
            "OkHttpClient*.kt"
        )
    }
    server = root / f"{core}/test/kotlin/{package}/TestServerExtension.kt"
    return found | ({server.relative_to(root).as_posix()} if server.is_file() else set())


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_covers_every_runtime_file(oracle: Oracle) -> None:
    root = _root(oracle)
    rendered = set(render_java_files(oracle.options))
    generated = {oracle.path(rel) for rel in (*GENERATED, *oracle.generated)}
    expected = _on_disk(root, oracle) - generated
    assert expected - rendered == set()
    assert rendered - expected <= _drift(oracle)
