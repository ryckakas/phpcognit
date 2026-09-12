#!/usr/bin/env bash
#
# Installs every PHP cognitive-complexity tool into throwaway Composer projects,
# scores benchmark/cases.php with each, then times each over a corpus.
#
#   ./benchmark/run.sh                      # scores only
#   ./benchmark/run.sh /path/to/php/source  # scores and timings
#
# Needs php, composer, and a release build of phpcognit.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
work="${BENCH_WORK:-$(mktemp -d)}"
corpus="${1:-}"

phpcognit="$root/target/release/phpcognit"
[ -x "$phpcognit" ] || { echo "build first: cargo build --release" >&2; exit 1; }

install_tool() {
    local name="$1" manifest="$2"
    mkdir -p "$work/$name/src"
    cp "$here/cases.php" "$work/$name/src/"
    printf '%s' "$manifest" > "$work/$name/composer.json"
    (cd "$work/$name" && composer install --quiet --no-interaction)
}

echo "workspace: $work"
echo

install_tool ncac '{
  "require-dev": { "ncac/php-cognitive-complexity": "*" }
}'

install_tool rarst '{
  "require-dev": {
    "squizlabs/php_codesniffer": "^3.7",
    "rarst/phpcs-cognitive-complexity": "*",
    "dealerdirect/phpcodesniffer-composer-installer": "^1.0"
  },
  "config": { "allow-plugins": { "dealerdirect/phpcodesniffer-composer-installer": true } }
}'

install_tool votruba '{
  "require-dev": {
    "phpstan/phpstan": "^2.0",
    "phpstan/extension-installer": "^1.4",
    "tomasvotruba/cognitive-complexity": "^1.0"
  },
  "config": { "allow-plugins": { "phpstan/extension-installer": true } }
}'

cat > "$work/rarst/phpcs.xml" <<'XML'
<?xml version="1.0"?>
<ruleset name="bench">
    <rule ref="CognitiveComplexity.Complexity.MaximumComplexity">
        <properties><property name="maxCognitiveComplexity" value="1"/></properties>
    </rule>
</ruleset>
XML

cat > "$work/votruba/phpstan.neon" <<'NEON'
parameters:
    level: 0
    cognitive_complexity:
        class: 5000
        function: 1
NEON

# Every one of these exits non-zero when it finds violations, which is exactly
# what they are being asked to do here, so none of it should abort the run.
set +e

echo "=== scores on cases.php ==="
echo
echo "-- phpcognit --"
"$phpcognit" --all --over 9999 "$here/cases.php" | awk '{printf "%-6s %s\n", $1, $3}'

echo
echo "-- ncac --"
(cd "$work/ncac" && ./vendor/bin/cc --max=1 --format=json src 2>/dev/null) \
    | python3 -c 'import json,sys
for v in json.load(sys.stdin)["violations"]:
    print("%-6s Cases::%s" % (v["score"], v["function"]))'

echo
echo "-- Rarst (phpcs) --"
(cd "$work/rarst" && ./vendor/bin/phpcs --standard=phpcs.xml --report=csv src 2>/dev/null) \
    | python3 -c 'import re,sys
for line in sys.stdin:
    m = re.search(r"for [\\\\\"]*([A-Za-z_]+)[\\\\\"]* is (\d+)", line)
    if m:
        print("%-6s Cases::%s" % (m.group(2), m.group(1)))'

echo
echo "-- TomasVotruba (phpstan) --"
(cd "$work/votruba" && ./vendor/bin/phpstan analyse -c phpstan.neon --no-progress --error-format=raw src 2>/dev/null) \
    | grep -i cognitive \
    | sed -E 's/.*"Cases::([a-zA-Z]+)\(\)" is ([0-9]+).*/\2      Cases::\1/' || true

[ -n "$corpus" ] || { echo; echo "pass a corpus path for timings"; exit 0; }

echo
echo "=== timings over $corpus ==="
echo "files: $(find "$corpus" -name '*.php' | wc -l | tr -d ' ')"
echo

time_it() {
    local label="$1"; shift
    local start end
    start=$(python3 -c 'import time;print(time.time())')
    "$@" >/dev/null 2>&1 || true
    end=$(python3 -c 'import time;print(time.time())')
    printf "%-38s %7.2fs\n" "$label" "$(echo "$end - $start" | bc)"
}

time_it "phpcognit" "$phpcognit" --over 15 "$corpus"
(cd "$work/ncac" && time_it "ncac/php-cognitive-complexity" ./vendor/bin/cc --max=15 "$corpus")
(cd "$work/rarst" && time_it "Rarst/phpcs-cognitive-complexity" ./vendor/bin/phpcs --standard=phpcs.xml "$corpus")
(cd "$work/votruba" && time_it "tomasvotruba/cognitive-complexity" \
    ./vendor/bin/phpstan analyse -c phpstan.neon --no-progress --error-format=raw --memory-limit=2G "$corpus")
