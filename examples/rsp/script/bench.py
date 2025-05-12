#!/usr/bin/env python3
# benchmark.py <N> <first-layer|two-to-one> [concurrency] [address]
#
# Sends N proof requests to the server (first-layer or two-to-one) and measures
# the total time until all return "ok". Requests are sent concurrently without
# blocking, respecting a concurrency limit. Validates proof files for two-to-one.
#
# Usage:
#   ./benchmark.py 10 first-layer 4 127.0.0.1:3000
#   ./benchmark.py 5 two-to-one

import argparse
import concurrent.futures
import json
import os
import sys
import time
import requests
import logging

# Set up logging
logging.basicConfig(level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s")
logger = logging.getLogger(__name__)

PREFIX = "./proofs/"

def url(base, path):
    return f"http://{base}{path}"

def do_request(i, request_type, address, max_retries=3):
    for attempt in range(1, max_retries + 1):
        try:
            if request_type == "first-layer":
                logger.debug(f"Request {i}: Sending POST to /first-layer/{i}")
                response = requests.post(url(address, f"/first-layer/{i}"), timeout=30)
            else:
                idx1 = i * 2
                idx2 = i * 2 + 1
                data = {"index1": idx1, "index2": idx2}
                logger.debug(f"Request {i}: Sending POST to /two-to-one with {data}")
                response = requests.post(
                    url(address, "/two-to-one"),
                    json=data,
                    headers={"Content-Type": "application/json"},
                    timeout=30
                )
            if response.status_code != 200:
                return i, False, f"status {response.status_code}, body: {response.text}"
            if response.text.strip() != "ok":
                return i, False, f"non-ok body: {response.text}"
            logger.debug(f"Request {i}: Succeeded")
            return i, True, None
        except requests.RequestException as e:
            error_msg = f"request error (attempt {attempt}/{max_retries}): {str(e)}"
            if attempt == max_retries:
                return i, False, error_msg
            logger.warning(f"Request {i}: {error_msg}, retrying...")
            time.sleep(1)  # Backoff before retry

def main():
    parser = argparse.ArgumentParser(description="Benchmark server proof requests")
    parser.add_argument("N", type=int, help="Number of proof requests")
    parser.add_argument("type", choices=["first-layer", "two-to-one"], help="Request type")
    parser.add_argument("concurrency", type=int, nargs="?", default=os.cpu_count() or 4, help="Max concurrent requests")
    parser.add_argument("address", nargs="?", default="127.0.0.1:3000", help="Server address")
    args = parser.parse_args()

    N = args.N
    request_type = args.type
    concurrency = args.concurrency
    address = args.address

    print(f"Benchmark: type={request_type}, N={N}, concurrency={concurrency}, addr={address}")

    # Validate inputs
    if N <= 0:
        print("Error: N must be a positive integer", file=sys.stderr)
        sys.exit(1)
    if concurrency <= 0:
        print("Error: concurrency must be a positive integer", file=sys.stderr)
        sys.exit(1)

    # Check proof files for two-to-one
    if request_type == "two-to-one":
        for i in range(N):
            idx1 = i * 2
            idx2 = i * 2 + 1
            if not (os.path.isfile(f"{PREFIX}reduced_0_{idx1}.bin") and
                    os.path.isfile(f"{PREFIX}reduced_0_{idx2}.bin")):
                print(f"Error: missing proof files reduced_0_{idx1}.bin or reduced_0_{idx2}.bin", file=sys.stderr)
                sys.exit(1)

    start_time = time.perf_counter()
    success_count = 0
    errors = []

    # Send requests concurrently
    with concurrent.futures.ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = [executor.submit(do_request, i, request_type, address) for i in range(N)]
        for future in concurrent.futures.as_completed(futures):
            i, success, error = future.result()
            if success:
                success_count += 1
            else:
                errors.append(f"Request {i} failed: {error}")

    end_time = time.perf_counter()
    elapsed_s = end_time - start_time
    elapsed_ms = elapsed_s * 1000

    if success_count != N or errors:
        print(f"❌ {success_count} of {N} requests succeeded", file=sys.stderr)
        for error in errors:
            print(error, file=sys.stderr)
        sys.exit(1)

    print(f"✅ {N} requests completed in {elapsed_s:.3f}s ({elapsed_ms:.0f} ms)")

if __name__ == "__main__":
    main()
