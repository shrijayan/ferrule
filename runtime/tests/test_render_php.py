# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the PHP runtime: namespace, name, client settings and import order."""

from __future__ import annotations

import pytest

from ferrule_runtime import PhpOptions, RenderError, render_php_files, template_root
from ferrule_runtime._php import sort_uses

ACME = PhpOptions(namespace="Acme")


def test_template_root_exists() -> None:
    assert template_root("php").joinpath("src", "Core", "BaseClient.php").is_file()


def test_php_defaults() -> None:
    files = render_php_files(ACME)
    base = files["src/Core/Exceptions/AcmeException.php"]
    assert "namespace Acme\\Core\\Exceptions;\n" in base
    assert "class AcmeException extends \\Exception\n" in base
    assert "protected const DESC = 'Acme Error';" in base
    assert (
        "class APIException extends AcmeException\n"
        in files["src/Core/Exceptions/APIException.php"]
    )
    options = files["src/RequestOptions.php"]
    assert "namespace Acme;\n" in options
    assert "public float $timeout = 60;\n" in options
    assert "public int $maxRetries = 2;\n" in options
    assert "public float $initialRetryDelay = 0.5;\n" in options
    assert "public float $maxRetryDelay = 8.0;\n" in options
    assert not any("ferrule" in rel.lower() or "ferrule" in t.lower() for rel, t in files.items())


def test_client_settings_and_name() -> None:
    files = render_php_files(
        PhpOptions(
            namespace="Acme\\Sdk",
            name="AcmeSdk",
            timeout=30.5,
            max_retries=5,
            initial_retry_delay=1,
            max_retry_delay=60,
        )
    )
    options = files["src/RequestOptions.php"]
    assert "namespace Acme\\Sdk;\n" in options
    assert "public float $timeout = 30.5;\n" in options
    assert "public int $maxRetries = 5;\n" in options
    assert "public float $initialRetryDelay = 1.0;\n" in options
    assert "public float $maxRetryDelay = 60.0;\n" in options
    assert "'AcmeSdk Not Found Exception'" in files["src/Core/Exceptions/NotFoundException.php"]


def test_uses_sort_after_the_namespace() -> None:
    # php-cs-fixer sorts `Zeta\...` after `Psr\...`; the template has them the other way round.
    text = render_php_files(PhpOptions(namespace="Zeta"))["src/RequestOptions.php"]
    uses = [line for line in text.splitlines() if line.startswith("use ")]
    assert uses == sorted(uses, key=lambda u: u.replace("\\", " ").lower())
    assert uses[0] == "use Psr\\Http\\Client\\ClientInterface;"
    assert uses[-1] == "use Zeta\\Core\\Contracts\\BaseModel;"


def test_sort_uses_keeps_blocks_apart() -> None:
    text = "use B;\nuse a;\n\nfinal class X\n{\n    use Z;\n    use Y;\n}\nuse D;\nuse C;\n"
    assert sort_uses(text) == (
        "use a;\nuse B;\n\nfinal class X\n{\n    use Z;\n    use Y;\n}\nuse C;\nuse D;\n"
    )


@pytest.mark.parametrize(
    "options",
    [
        PhpOptions(namespace="acme"),
        PhpOptions(namespace="Acme\\"),
        PhpOptions(namespace="Acme", name="Acme Sdk"),
        PhpOptions(namespace="Acme", timeout=0),
        PhpOptions(namespace="Acme", max_retries=-1),
        PhpOptions(namespace="Acme", max_retry_delay=-1),
    ],
)
def test_invalid_options(options: PhpOptions) -> None:
    with pytest.raises(RenderError):
        render_php_files(options)
