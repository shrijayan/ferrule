---
name: onebusaway
description: Use the onebusaway Python SDK (version 0.0.1) to call the OnebusawaySDK API: client setup, authentication, one example per resource, errors and retries.
---

# onebusaway 0.0.1

This skill ships inside the `onebusaway` package, so it matches the installed version. The full method and type reference is `api.md` in the repository (https://github.com/OneBusAway/python-sdk).

## Install

```sh
pip install onebusaway==0.0.1
```

## Authenticate and create a client

Set `ONEBUSAWAY_API_KEY`; the client reads it when `api_key` is not passed. Never hard-code the key.

```python
import os

from onebusaway import OnebusawaySDK

client = OnebusawaySDK(
    api_key=os.environ.get("ONEBUSAWAY_API_KEY"),  # the default
    max_retries=2,  # the default; connection errors, 408, 409, 429 and 5xx are retried
)
```

Use `AsyncOnebusawaySDK` the same way, with `await` on every call. `ONEBUSAWAY_SDK_BASE_URL` overrides the base URL.

## Examples

One call per resource; the arguments are placeholders.

### `client.agencies_with_coverage.list` (GET /api/where/agencies-with-coverage.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

agencies_with_coverage = client.agencies_with_coverage.list()
print(agencies_with_coverage)
```

### `client.agency.retrieve` (GET /api/where/agency/{agencyID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

agency = client.agency.retrieve(agency_id="agencyID")
print(agency)
```

### `client.vehicles_for_agency.list` (GET /api/where/vehicles-for-agency/{agencyID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

vehicles_for_agency = client.vehicles_for_agency.list(agency_id="agencyID")
print(vehicles_for_agency)
```

### `client.config.retrieve` (GET /api/where/config.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

config = client.config.retrieve()
print(config)
```

### `client.current_time.retrieve` (GET /api/where/current-time.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

current_time = client.current_time.retrieve()
print(current_time)
```

### `client.stops_for_location.list` (GET /api/where/stops-for-location.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

stops_for_location = client.stops_for_location.list()
print(stops_for_location)
```

### `client.stops_for_route.list` (GET /api/where/stops-for-route/{routeID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

stops_for_route = client.stops_for_route.list(route_id="routeID")
print(stops_for_route)
```

### `client.stops_for_agency.list` (GET /api/where/stops-for-agency/{agencyID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

stops_for_agency = client.stops_for_agency.list(agency_id="agencyID")
print(stops_for_agency)
```

### `client.stop.retrieve` (GET /api/where/stop/{stopID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

stop = client.stop.retrieve(stop_id="stopID")
print(stop)
```

### `client.stop_ids_for_agency.list` (GET /api/where/stop-ids-for-agency/{agencyID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

stop_ids_for_agency = client.stop_ids_for_agency.list(agency_id="agencyID")
print(stop_ids_for_agency)
```

### `client.schedule_for_stop.retrieve` (GET /api/where/schedule-for-stop/{stopID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

schedule_for_stop = client.schedule_for_stop.retrieve(stop_id="stopID")
print(schedule_for_stop)
```

### `client.route.retrieve` (GET /api/where/route/{routeID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

route = client.route.retrieve(route_id="routeID")
print(route)
```

### `client.route_ids_for_agency.list` (GET /api/where/route-ids-for-agency/{agencyID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

route_ids_for_agency = client.route_ids_for_agency.list(agency_id="agencyID")
print(route_ids_for_agency)
```

### `client.routes_for_location.list` (GET /api/where/routes-for-location.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

routes_for_location = client.routes_for_location.list()
print(routes_for_location)
```

### `client.routes_for_agency.list` (GET /api/where/routes-for-agency/{agencyID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

routes_for_agency = client.routes_for_agency.list(agency_id="agencyID")
print(routes_for_agency)
```

### `client.schedule_for_route.retrieve` (GET /api/where/schedule-for-route/{routeID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

schedule_for_route = client.schedule_for_route.retrieve(route_id="routeID")
print(schedule_for_route)
```

### `client.arrivals_and_departures_for_location.list` (GET /api/where/arrivals-and-departures-for-location.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

arrivals_and_departures_for_location = client.arrivals_and_departures_for_location.list(lat=0, lon=0)
print(arrivals_and_departures_for_location)
```

### `client.arrival_and_departure.retrieve` (GET /api/where/arrival-and-departure-for-stop/{stopID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

arrival_and_departure = client.arrival_and_departure.retrieve(stop_id="stopID", service_date=0, trip_id="tripId")
print(arrival_and_departure)
```

### `client.trip.retrieve` (GET /api/where/trip/{tripID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

trip = client.trip.retrieve(trip_id="tripID")
print(trip)
```

### `client.trips_for_location.list` (GET /api/where/trips-for-location.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

trips_for_location = client.trips_for_location.list(lat_span=0, lon_span=0)
print(trips_for_location)
```

### `client.trip_details.retrieve` (GET /api/where/trip-details/{tripID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

trip_details = client.trip_details.retrieve(trip_id="tripID")
print(trip_details)
```

### `client.trip_for_vehicle.retrieve` (GET /api/where/trip-for-vehicle/{vehicleID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

trip_for_vehicle = client.trip_for_vehicle.retrieve(vehicle_id="vehicleID")
print(trip_for_vehicle)
```

### `client.trips_for_route.list` (GET /api/where/trips-for-route/{routeID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

trips_for_route = client.trips_for_route.list(route_id="routeID")
print(trips_for_route)
```

### `client.report_problem_with_stop.retrieve` (GET /api/where/report-problem-with-stop/{stopID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

report_problem_with_stop = client.report_problem_with_stop.retrieve(stop_id="stopID")
print(report_problem_with_stop)
```

### `client.report_problem_with_trip.retrieve` (GET /api/where/report-problem-with-trip/{tripID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

report_problem_with_trip = client.report_problem_with_trip.retrieve(trip_id="tripID")
print(report_problem_with_trip)
```

### `client.search_for_stop.list` (GET /api/where/search/stop.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

search_for_stop = client.search_for_stop.list(input="input")
print(search_for_stop)
```

### `client.search_for_route.list` (GET /api/where/search/route.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

search_for_route = client.search_for_route.list(input="input")
print(search_for_route)
```

### `client.block.retrieve` (GET /api/where/block/{blockID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

block = client.block.retrieve(block_id="blockID")
print(block)
```

### `client.shape.retrieve` (GET /api/where/shape/{shapeID}.json)

```python
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

shape = client.shape.retrieve(shape_id="shapeID")
print(shape)
```

## Errors

```python
import onebusaway
from onebusaway import OnebusawaySDK

client = OnebusawaySDK()

try:
    agencies_with_coverage = client.agencies_with_coverage.list()
    print(agencies_with_coverage)
except onebusaway.APIConnectionError as e:
    print("The server could not be reached:", e.__cause__)
except onebusaway.RateLimitError:
    print("Rate limited (429); back off and retry.")
except onebusaway.APIStatusError as e:
    print("Non-2xx response:", e.status_code, e.response)
```

Every method also has `.with_raw_response` (headers and the raw response) and `.with_streaming_response`.
