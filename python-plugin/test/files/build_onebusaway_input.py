# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
# ruff: noqa: INP001 -- a standalone dev script, not part of a package
"""Builds test/files/onebusaway-input.json, the ProjectFilesInput of the OneBusAway oracle.

Run: python3 -I python-plugin/test/files/build_onebusaway_input.py (after
`python3 oracles/fetch.py onebusaway`). Parameters and types come from the paired spec and
config; the names SdkModel will supply (resource titles, result variables, type names and
modules) are read from the oracle's api.md, standing in for the code emitters until the lead
wires the real mapping.
"""

import json
import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
CFG = ROOT / "oracles/cache/onebusaway-config"
SDK = ROOT / "oracles/cache/onebusaway-python"
OUT = Path(__file__).with_name("onebusaway-input.json")


def snake(name: str) -> str:
    """Python spelling of a wire name: `stopID` -> `stop_id`."""
    s = re.sub(r"ID", "Id", name)
    return re.sub(r"(?<!^)(?=[A-Z])", "_", s).lower()


def sdk_type(schema: dict) -> dict:
    """Return the SdkType of a parameter schema (the shapes OneBusAway uses)."""
    if "enum" in schema:
        return {"kind": "literal", "values": schema["enum"]}
    t = schema.get("type")
    if t == "string" and schema.get("format") == "date-time":
        return {"kind": "primitive", "name": "datetime"}
    if t == "string" and schema.get("format") == "date":
        return {"kind": "primitive", "name": "date"}
    if t in ("string", "integer", "number", "boolean"):
        return {"kind": "primitive", "name": t}
    return {"kind": "unknown"}


def parse_api_md() -> dict:
    """Resource title -> its api.md types and method lines."""
    text = (SDK / "api.md").read_text()
    out = {}
    for block in re.split(r"^# ", text, flags=re.MULTILINE)[1:]:
        title = block.splitlines()[0]
        types = re.search(r"from onebusaway\.types import (.+)", block)
        methods = re.findall(
            r'<code title="(\w+) ([^"]+)">client\.(\w+)\.<a href="[^"]+">(\w+)</a>\(([^)]*)\) -> '
            r'<a href="\./src/onebusaway/([^"]+)\.py">(\w+)</a>',
            block,
        )
        out[title] = {"types": types.group(1).split(", ") if types else [], "methods": methods}
    return out


def variable(sdk_test: Path) -> str:
    """Return the result variable the oracle test module uses."""
    m = re.search(r"^        (\w+) = client\.", sdk_test.read_text(), flags=re.MULTILINE)
    if m is None:
        raise ValueError(sdk_test)
    return m.group(1)


def main() -> None:
    """Write onebusaway-input.json."""
    spec = yaml.safe_load((CFG / "openapi.yml").read_text())
    config = yaml.safe_load((CFG / "stainless-config.yml").read_text())
    api = parse_api_md()
    resources = []
    for title, entry in api.items():
        if title == "Shared Types":
            continue
        name = entry["methods"][0][2]
        methods = []
        for verb, path, _, method, sig, resp_mod, resp_name in entry["methods"]:
            op = spec["paths"][path][verb]
            params = []
            for prm in spec["paths"][path].get("parameters", []) + op.get("parameters", []):
                p = {
                    "pyName": snake(prm["name"]),
                    "wireName": prm["name"],
                    "required": bool(prm.get("required", False)),
                    "location": prm["in"],
                    "type": sdk_type(prm["schema"]),
                }
                for key in ("example",):
                    if key in prm:
                        p[key] = prm[key]
                for key in ("minimum", "maximum"):
                    if key in prm["schema"]:
                        p[key] = prm["schema"][key]
                params.append(p)
            params.sort(key=lambda p: p["location"] != "path")
            pm = re.search(r'href="src/onebusaway/([^"]+)\.py">params', sig)
            entry_out = {
                "name": method,
                "httpMethod": verb,
                "path": path,
                "params": params,
                "response": {"name": resp_name, "module": resp_mod.replace("/", ".")},
            }
            if pm:
                entry_out["paramsModule"] = pm.group(1).replace("/", ".")
            methods.append(entry_out)
        resources.append(
            {
                "name": name,
                "title": title,
                "variable": variable(SDK / f"tests/api_resources/test_{name}.py"),
                "module": f"resources.{name}",
                "types": entry["types"],
                "typesModule": "types",
                "methods": methods,
                "subresources": [],
            }
        )
    stats = dict(line.split(": ", 1) for line in (SDK / ".stats.yml").read_text().splitlines())
    org = config["organization"]
    data = {
        "edition": "python.2025-10-08",
        "packageName": "onebusaway",
        "distributionName": config["targets"]["python"]["package_name"],
        "clientClass": "OnebusawaySDK",
        "envPrefix": "ONEBUSAWAY_SDK",
        "repo": config["targets"]["python"]["production_repo"],
        "version": "1.30.0",
        "copyrightYear": 2026,
        "organization": {
            "name": org["name"],
            "title": "Onebusaway SDK",
            "contact": org["contact"],
            "docsUrl": org["docs"],
        },
        "readmeTitle": "OneBusAway SDK",
        "auth": {
            "kind": "api_key_query",
            "env": config["client_settings"]["opts"]["api_key"]["read_env"],
            "param": config["client_settings"]["opts"]["api_key"]["send_as_query_param"],
        },
        "query": {
            "arrayFormat": config["query_settings"]["array_format"],
            "objectFormat": config["query_settings"]["nested_format"],
        },
        "stats": {
            "openapiSpecUrl": stats["openapi_spec_url"],
            "openapiSpecHash": stats["openapi_spec_hash"],
            "configHash": stats["config_hash"],
        },
        "readmeExample": {
            "accessor": "current_time",
            "method": "retrieve",
            "args": "",
            "httpMethod": "get",
            "path": "/api/where/current-time.json",
        },
        "retryProbe": {
            "accessor": "current_time",
            "method": "retrieve",
            "args": "",
            "httpMethod": "get",
            "path": "/api/where/current-time.json",
        },
        "sharedTypes": ["References", "ResponseWrapper"],
        "resources": resources,
    }
    OUT.write_text(json.dumps(data, indent=2) + "\n")
    sys.stdout.write(f"wrote {OUT}\n")


if __name__ == "__main__":
    main()
