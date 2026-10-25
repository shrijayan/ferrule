//! The generated JVM files against real Stainless SDKs, byte for byte after ktfmt and the
//! branding substitutions (`common::brand`). Skipped when `oracles/cache` is not fetched or
//! ktfmt is not installed (`mise install`).
//!
//! onebusaway-java (`c29c48ad`) and onebusaway-kotlin (`69234467`), both Apache-2.0, are the same
//! generation as the proven Python pin: their `.stats.yml` hashes equal the md5 of the config
//! oracle's `openapi.yml` and `stainless-config.yml`.

mod common;

use std::path::{Path, PathBuf};

use ferrule_jvm_plugin::{Flavor, JvmMeta, generate};

struct Oracle {
    name: &'static str,
    flavor: Flavor,
    spec: &'static str,
    config: &'static str,
    version: &'static str,
    /// `readme.example_requests.default` (not in the model yet): resource key and method.
    example: (&'static str, &'static str),
    /// Files (path suffixes under the package directory) that differ for a recorded reason, on
    /// top of [`G18`]. The test fails when one of them starts matching, so the list stays true.
    known: &'static [(&'static str, &'static str)],
}

const G18_PATH: &str = "Forge gap G18: the path param's `example` (1_75403, 1_100223, 40) is not in the model";
const G18_MAX: &str = "Forge gap G18: `maxCount`'s `maximum` (1000) is not in the model";
const G18_FIELD: &str = "Forge gap G18: the response fields' `example`s (1_100224, 1_10911) are not in the model";
const KDOC: &str = "Stainless spelled the raw `list` setter's KDoc type `kotlin.collections.List<List>` in this \
                    file only; StopsForLocationListResponse.kt, the same class, has `List<List>`";
const HAND: &str = "hand edit in OneBusAway/kotlin-sdk (custom code): `limitExceeded`/`outOfRange` always read \
                    and the `Situation.Reason` enum; onebusaway-java, the same generation, has the spec's shape";

/// Files the G18 gap changes, in both flavors.
const G18: &[(&str, &str)] = &[
    (
        "models/arrivalanddeparture/ArrivalAndDepartureListParamsTest.kt",
        G18_PATH,
    ),
    (
        "models/arrivalanddeparture/ArrivalAndDepartureRetrieveParamsTest.kt",
        G18_PATH,
    ),
    (
        "models/arrivalsanddeparturesforlocation/ArrivalsAndDeparturesForLocationListParamsTest.kt",
        G18_MAX,
    ),
    ("models/routesforagency/RoutesForAgencyListParamsTest.kt", G18_PATH),
    (
        "models/scheduleforroute/ScheduleForRouteRetrieveParamsTest.kt",
        G18_PATH,
    ),
    ("services/async/ArrivalAndDepartureServiceAsyncTest.kt", G18_PATH),
    (
        "services/async/ArrivalsAndDeparturesForLocationServiceAsyncTest.kt",
        G18_MAX,
    ),
    ("services/async/RoutesForAgencyServiceAsyncTest.kt", G18_PATH),
    ("services/async/ScheduleForRouteServiceAsyncTest.kt", G18_PATH),
    ("services/blocking/ArrivalAndDepartureServiceTest.kt", G18_PATH),
    (
        "services/blocking/ArrivalsAndDeparturesForLocationServiceTest.kt",
        G18_MAX,
    ),
    ("services/blocking/RoutesForAgencyServiceTest.kt", G18_PATH),
    ("services/blocking/ScheduleForRouteServiceTest.kt", G18_PATH),
];

const JAVA_KNOWN: &[(&str, &str)] = &[
    ("models/stopsforagency/StopsForAgencyListResponse.kt", KDOC),
    ("models/stopsforroute/StopsForRouteListResponseTest.kt", G18_FIELD),
];

const KOTLIN_KNOWN: &[(&str, &str)] = &[
    ("models/References.kt", HAND),
    ("models/agencieswithcoverage/AgenciesWithCoverageListResponse.kt", HAND),
    ("models/agency/AgencyRetrieveResponse.kt", HAND),
    ("models/routeidsforagency/RouteIdsForAgencyListResponse.kt", HAND),
    ("models/routesforagency/RoutesForAgencyListResponse.kt", HAND),
    ("models/routesforlocation/RoutesForLocationListResponse.kt", HAND),
    ("models/searchforroute/SearchForRouteListResponse.kt", HAND),
    ("models/searchforstop/SearchForStopListResponse.kt", HAND),
    ("models/stopidsforagency/StopIdsForAgencyListResponse.kt", HAND),
    ("models/stopsforagency/StopsForAgencyListResponse.kt", HAND),
    ("models/stopsforlocation/StopsForLocationListResponse.kt", HAND),
    ("models/tripsforlocation/TripsForLocationListResponse.kt", HAND),
    ("models/tripsforroute/TripsForRouteListResponse.kt", HAND),
    ("models/vehiclesforagency/VehiclesForAgencyListResponse.kt", HAND),
    // Every test that builds a `References` or a hand-edited response carries the hand edit.
    ("models/ReferencesTest.kt", HAND),
    (
        "models/agencieswithcoverage/AgenciesWithCoverageListResponseTest.kt",
        HAND,
    ),
    ("models/agency/AgencyRetrieveResponseTest.kt", HAND),
    (
        "models/arrivalanddeparture/ArrivalAndDepartureListResponseTest.kt",
        HAND,
    ),
    (
        "models/arrivalanddeparture/ArrivalAndDepartureRetrieveResponseTest.kt",
        HAND,
    ),
    (
        "models/arrivalsanddeparturesforlocation/ArrivalsAndDeparturesForLocationListResponseTest.kt",
        HAND,
    ),
    ("models/block/BlockRetrieveResponseTest.kt", HAND),
    ("models/config/ConfigRetrieveResponseTest.kt", HAND),
    ("models/currenttime/CurrentTimeRetrieveResponseTest.kt", HAND),
    ("models/route/RouteRetrieveResponseTest.kt", HAND),
    ("models/routeidsforagency/RouteIdsForAgencyListResponseTest.kt", HAND),
    ("models/routesforagency/RoutesForAgencyListResponseTest.kt", HAND),
    ("models/routesforlocation/RoutesForLocationListResponseTest.kt", HAND),
    ("models/scheduleforstop/ScheduleForStopRetrieveResponseTest.kt", HAND),
    ("models/searchforroute/SearchForRouteListResponseTest.kt", HAND),
    ("models/searchforstop/SearchForStopListResponseTest.kt", HAND),
    ("models/shape/ShapeRetrieveResponseTest.kt", HAND),
    ("models/stop/StopRetrieveResponseTest.kt", HAND),
    ("models/stopidsforagency/StopIdsForAgencyListResponseTest.kt", HAND),
    ("models/stopsforagency/StopsForAgencyListResponseTest.kt", HAND),
    ("models/stopsforlocation/StopsForLocationListResponseTest.kt", HAND),
    ("models/stopsforroute/StopsForRouteListResponseTest.kt", HAND),
    ("models/trip/TripRetrieveResponseTest.kt", HAND),
    ("models/tripdetails/TripDetailRetrieveResponseTest.kt", HAND),
    ("models/tripforvehicle/TripForVehicleRetrieveResponseTest.kt", HAND),
    ("models/tripsforlocation/TripsForLocationListResponseTest.kt", HAND),
    ("models/tripsforroute/TripsForRouteListResponseTest.kt", HAND),
    ("models/vehiclesforagency/VehiclesForAgencyListResponseTest.kt", HAND),
    ("proguard/ProGuardCompatibilityTest.kt", HAND),
];

const ONEBUSAWAY_KOTLIN: Oracle = Oracle {
    name: "onebusaway-kotlin",
    flavor: Flavor::Kotlin,
    spec: "onebusaway-config/openapi.yml",
    config: "onebusaway-config/stainless-config.yml",
    version: "0.1.0-alpha.81",
    example: ("current_time", "retrieve"),
    known: KOTLIN_KNOWN,
};

const ONEBUSAWAY_JAVA: Oracle = Oracle {
    name: "onebusaway-java",
    flavor: Flavor::Java,
    spec: "onebusaway-config/openapi.yml",
    config: "onebusaway-config/stainless-config.yml",
    version: "0.1.0-alpha.60",
    example: ("current_time", "retrieve"),
    known: JAVA_KNOWN,
};

fn run(o: &Oracle, scratch: &str) -> Option<(PathBuf, Vec<(String, String)>)> {
    let oracle = common::oracle(o.name)?;
    let annotated = common::annotated(o.spec, o.config)?;
    let meta = JvmMeta {
        version: Some(o.version.into()),
        example: Some((vec![o.example.0.into()], o.example.1.into())),
        year: Some(2026),
        docs_url: Some("https://developer.onebusaway.org".into()),
        contact: Some("info@onebusaway.org".into()),
        ..JvmMeta::default()
    };
    let out = generate(&common::model(&annotated), o.flavor, &meta).expect("generates");
    let formatted = common::ktfmt(&out.files, &format!("{}-{scratch}", o.name))?;
    common::dump("FERRULE_JVM_DUMP", &formatted);
    Some((oracle, formatted))
}

/// The recorded reason a file differs, if any.
fn known(o: &Oracle, path: &str) -> Option<&'static str> {
    G18.iter()
        .chain(o.known)
        .find(|(suffix, _)| path.ends_with(&format!("/org/onebusaway/{suffix}")))
        .map(|(_, reason)| *reason)
}

fn compare(o: &Oracle) {
    let Some((oracle, files)) = run(o, "compare") else {
        return;
    };
    let mut identical = 0;
    let mut recorded = 0;
    let mut differing = Vec::new();
    let mut missing = Vec::new();
    for (path, text) in &files {
        let Ok(raw) = std::fs::read_to_string(oracle.join(path)) else {
            missing.push(path.clone());
            continue;
        };
        let expected = common::brand(path, &raw);
        match (text == &expected, known(o, path)) {
            (true, None) => identical += 1,
            (true, Some(_)) => panic!("{path} is a known difference but now matches; drop it"),
            (false, Some(_)) => recorded += 1,
            (false, None) => differing.push(path.clone()),
        }
    }
    eprintln!("{}: {identical}/{} generated files identical", o.name, files.len());
    if let Ok(list) = std::env::var("FERRULE_JVM_REPORT") {
        let report = format!("differ:\n{}\nmissing:\n{}\n", differing.join("\n"), missing.join("\n"));
        std::fs::write(Path::new(&list), report).expect("report");
    }
    assert!(missing.is_empty(), "not in the oracle: {missing:?}");
    assert!(differing.is_empty(), "differ from the oracle: {differing:?}");
    assert_eq!(
        recorded,
        G18.len() + o.known.len(),
        "a recorded difference is not generated"
    );
}

/// Oracle files the plugin does not generate: the runtime template's (`runtime/java`, proven by
/// `runtime/tests/test_render_oracles_java.py`), hand-written examples, release-please's
/// `CHANGELOG.md`, Stainless's upload script (dropped as branding), the binary Gradle wrapper and
/// stray files in onebusaway-java's root.
fn not_generated(path: &str) -> bool {
    let in_core = |dir: &str| {
        [
            "-core/src/main/kotlin/org/onebusaway/",
            "-core/src/test/kotlin/org/onebusaway/",
        ]
        .iter()
        .any(|base| path.contains(&format!("{base}{dir}")))
    };
    let runtime =
        (in_core("core/") && !path.ends_with("/ClientOptions.kt") && !path.ends_with("/ClientOptionsTest.kt"))
            || in_core("errors/")
            || path.ends_with("/TestServerExtension.kt")
            || path.ends_with("/OkHttpClient.kt")
            || path.ends_with("/OkHttpClientTest.kt")
            || path.contains("/META-INF/proguard/");
    let example = path.split('/').next().is_some_and(|m| m.ends_with("-example")) && path.contains("/src/");
    runtime
        || example
        || [
            "CHANGELOG.md",
            "scripts/upload-artifacts",
            "gradle/wrapper/gradle-wrapper.jar",
            "For",
            "Get",
            "Run",
        ]
        .contains(&path)
}

/// Project files the plugin does not emit yet.
fn not_yet(path: &str) -> bool {
    ["README.md", ".stats.yml"].contains(&path)
}

fn covers(o: &Oracle) {
    let Some((oracle, files)) = run(o, "covers") else {
        return;
    };
    let missing: Vec<String> = common::walk(&oracle)
        .into_iter()
        .filter(|p| !not_generated(p) && !not_yet(p))
        .filter(|p| !files.iter().any(|(f, _)| f == p))
        .collect();
    assert!(missing.is_empty(), "not emitted: {missing:?}");
}

/// imagekit-java (Apache-2.0, `f96c35e8`), the same generation as the proven imagekit-python pin.
/// Bodies, unions, uploads and webhooks are not all generated yet, so this test is a ratchet: it
/// fails when fewer files match than [`IMAGEKIT_IDENTICAL`].
const IMAGEKIT_JAVA: Oracle = Oracle {
    name: "imagekit-java",
    flavor: Flavor::Java,
    spec: "specs/imagekit-config.yml",
    config: "imagekit-config/stainless-config/main.yaml",
    version: "3.1.0",
    example: ("files", "upload"),
    known: &[],
};

const IMAGEKIT_IDENTICAL: usize = 186;

#[test]
fn imagekit_java_matches_at_least_the_ratchet() {
    let o = &IMAGEKIT_JAVA;
    let Some(oracle) = common::oracle(o.name) else { return };
    let Some(annotated) = common::annotated(o.spec, o.config) else {
        return;
    };
    let meta = JvmMeta {
        version: Some(o.version.into()),
        example: Some((vec![o.example.0.into()], o.example.1.into())),
        year: Some(2026),
        docs_url: Some("https://imagekit.io/docs/api-reference".into()),
        contact: Some("developer@imagekit.io".into()),
        mock_tests: Some(false),
        ..JvmMeta::default()
    };
    let out = generate(&common::model(&annotated), o.flavor, &meta).expect("generates");
    let Some(files) = common::ktfmt(&out.files, "imagekit-java-ratchet") else {
        return;
    };
    common::dump("FERRULE_JVM_DUMP_IMAGEKIT", &files);
    let mut identical = Vec::new();
    let mut differing = Vec::new();
    for (path, text) in &files {
        match std::fs::read_to_string(oracle.join(path)) {
            Ok(raw) if common::brand(path, &raw) == *text => identical.push(path.clone()),
            _ => differing.push(path.clone()),
        }
    }
    eprintln!(
        "imagekit-java: {}/{} generated files identical",
        identical.len(),
        files.len()
    );
    if let Ok(list) = std::env::var("FERRULE_JVM_REPORT_IMAGEKIT") {
        let report = format!(
            "identical:\n{}\ndiffer:\n{}\n",
            identical.join("\n"),
            differing.join("\n")
        );
        std::fs::write(Path::new(&list), report).expect("report");
    }
    assert!(
        identical.len() >= IMAGEKIT_IDENTICAL,
        "{} files match imagekit-java, fewer than the ratchet {IMAGEKIT_IDENTICAL}",
        identical.len()
    );
}

#[test]
fn onebusaway_kotlin_matches_the_oracle() {
    compare(&ONEBUSAWAY_KOTLIN);
}

#[test]
fn onebusaway_java_matches_the_oracle() {
    compare(&ONEBUSAWAY_JAVA);
}

#[test]
fn onebusaway_kotlin_emits_every_generated_file() {
    covers(&ONEBUSAWAY_KOTLIN);
}

#[test]
fn onebusaway_java_emits_every_generated_file() {
    covers(&ONEBUSAWAY_JAVA);
}
