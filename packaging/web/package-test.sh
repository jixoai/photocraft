#!/usr/bin/env bash
# Regression for generated module imports lost when packaging the app below app/.
# Uses only synthetic bytes; no Rust build, browser, network or user's files.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$(mktemp -d)"
trap 'rm -rf "$FIXTURE"' EXIT
mkdir -p "$FIXTURE/packaging/web" "$FIXTURE/docs" "$FIXTURE/dist/web/snippets/eframe-test" "$FIXTURE/dist/web-boot"
cp "$HERE/package.sh" "$HERE/_headers" "$HERE/.htaccess" "$HERE/README.md" "$FIXTURE/packaging/web/"
cp "$HERE/../env.sh" "$FIXTURE/packaging/"
cp "$HERE/../../docs/webmcp.md" "$FIXTURE/docs/"
cat > "$FIXTURE/Cargo.toml" <<'EOF'
[workspace.package]
version = "0.0.0"
EOF
printf '<html></html>\n' > "$FIXTURE/dist/web/index.html"
cp "$FIXTURE/dist/web/index.html" "$FIXTURE/dist/web-boot/index.html"
printf '\0asm\1\0\0\0' > "$FIXTURE/dist/web/photocraft-web-test_bg.wasm"
printf 'generated glue fixture\n' > "$FIXTURE/dist/web/photocraft-web-differenthash.js"
printf 'generated dependency module fixture\n' > "$FIXTURE/dist/web/snippets/eframe-test/inline0.js"
DIST="$FIXTURE/dist/release" CARGO_TARGET_DIR="$FIXTURE/target" PHOTOCRAFT_WEB_PART_BYTES=10 \
  bash "$FIXTURE/packaging/web/package.sh" --skip-build > "$FIXTURE/package.log"
unzip -q "$FIXTURE/dist/release/photocraft-web-0.0.0.zip" -d "$FIXTURE/unpacked"
SITE="$FIXTURE/unpacked/photocraft-web-0.0.0"
cmp "$FIXTURE/dist/web/snippets/eframe-test/inline0.js" "$SITE/app/snippets/eframe-test/inline0.js"
cmp "$FIXTURE/dist/web/photocraft-web-differenthash.js" "$SITE/app/photocraft-web.js"
cmp "$FIXTURE/docs/webmcp.md" "$SITE/WEBMCP.md"
[[ "$(head -1 "$SITE/app/parts.txt")" == "photocraft-parts 1" ]]
[[ "$(wc -l < "$SITE/app/parts.txt")" -gt 2 ]]
cat "$SITE/app/photocraft-web-test".part-* | gzip -dc > "$FIXTURE/reassembled.wasm"
cmp "$FIXTURE/dist/web/photocraft-web-test_bg.wasm" "$FIXTURE/reassembled.wasm"
cp "$FIXTURE/dist/web/photocraft-web-differenthash.js" "$FIXTURE/dist/web/photocraft-web-stale.js"
if DIST="$FIXTURE/dist/release" CARGO_TARGET_DIR="$FIXTURE/target" bash "$FIXTURE/packaging/web/package.sh" --skip-build > "$FIXTURE/stale.log" 2>&1; then
  echo 'FAIL: ambiguous stale loaders were accepted' >&2
  exit 1
fi
echo 'PASS: generated dependency modules, independent content hashes and multipart Wasm survive the release zip; stale loaders are rejected'
rm "$FIXTURE/dist/web/photocraft-web-stale.js"
mkdir -p "$FIXTURE/dist/web-threaded/snippets/worker-test"
cp "$FIXTURE/dist/web/index.html" "$FIXTURE/dist/web-threaded/index.html"
printf '\0asm\1\0\0\0threaded' > "$FIXTURE/dist/web-threaded/photocraft-web-worker_bg.wasm"
printf 'threaded generated glue fixture\n' > "$FIXTURE/dist/web-threaded/photocraft-web-otherhash.js"
printf 'generated worker helper fixture\n' > "$FIXTURE/dist/web-threaded/snippets/worker-test/worker.js"
DIST="$FIXTURE/dist/release" CARGO_TARGET_DIR="$FIXTURE/target" PHOTOCRAFT_WEB_PART_BYTES=10 \
  bash "$FIXTURE/packaging/web/package.sh" --skip-build > "$FIXTURE/threaded.log"
unzip -qo "$FIXTURE/dist/release/photocraft-web-0.0.0.zip" -d "$FIXTURE/unpacked"
cmp "$FIXTURE/dist/web-threaded/snippets/worker-test/worker.js" "$SITE/app-threaded/snippets/worker-test/worker.js"
cat "$SITE/app-threaded/photocraft-web-worker".part-* | gzip -dc > "$FIXTURE/threaded-reassembled.wasm"
cmp "$FIXTURE/dist/web-threaded/photocraft-web-worker_bg.wasm" "$FIXTURE/threaded-reassembled.wasm"
grep -q 'Cross-Origin-Opener-Policy: same-origin' "$SITE/_headers"
grep -q 'Cross-Origin-Embedder-Policy: require-corp' "$SITE/_headers"
echo 'PASS: worker build, generated worker modules, compatibility build and security headers coexist in the release zip'
mkdir -p "$FIXTURE/apps/photocraft-web" "$FIXTURE/bin"
cp "$HERE/build-threaded.sh" "$FIXTURE/packaging/web/"
cp "$HERE/../../apps/photocraft-web/index.html" "$FIXTURE/apps/photocraft-web/"
cp "$HERE/../../apps/photocraft-web/build.rs" "$FIXTURE/apps/photocraft-web/"
rustc --edition=2024 "$FIXTURE/apps/photocraft-web/build.rs" -o "$FIXTURE/thread-link-script"
CARGO_CFG_TARGET_ARCH=wasm32 CARGO_FEATURE_THREADS=1 "$FIXTURE/thread-link-script" > "$FIXTURE/thread-link.log"
for argument in --shared-memory --import-memory --export=__wasm_init_tls --export=__tls_size --export=__tls_align --export=__tls_base; do
  grep -q "cargo:rustc-link-arg-bin=photocraft-web=$argument" "$FIXTURE/thread-link.log"
done
CARGO_CFG_TARGET_ARCH=aarch64 CARGO_FEATURE_THREADS=1 "$FIXTURE/thread-link-script" > "$FIXTURE/native-link.log"
! grep -q rustc-link-arg "$FIXTURE/native-link.log"
env -u CARGO_FEATURE_THREADS CARGO_CFG_TARGET_ARCH=wasm32 "$FIXTURE/thread-link-script" > "$FIXTURE/compat-link.log"
! grep -q rustc-link-arg "$FIXTURE/compat-link.log"
printf '#!/usr/bin/env bash\nexit 0\n' > "$FIXTURE/bin/rustup"
cat > "$FIXTURE/bin/trunk" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == build ]]
grep -q 'data-cargo-features="heif,threads"' "$2"
[[ "$CARGO_UNSTABLE_BUILD_STD" == std,panic_abort ]]
[[ "$RUSTFLAGS" == *+atomics,+bulk-memory* ]]
[[ "$RUSTFLAGS" == *--max-memory=2147483648* ]]
output="${@: -1}"
mkdir -p "$output"
if [[ "${PHOTOCRAFT_TEST_NO_WORKER:-0}" == 1 ]]; then
  printf 'compatibility fixture\n' > "$output/photocraft-web-threaded.js"
else
  printf 'export function wbg_rayon_start_worker() {}\nconst memory = { shared: true };\n' > "$output/photocraft-web-threaded.js"
fi
EOF
chmod +x "$FIXTURE/bin/rustup" "$FIXTURE/bin/trunk"
PATH="$FIXTURE/bin:$PATH" bash "$FIXTURE/packaging/web/build-threaded.sh"
if compgen -G "$FIXTURE/apps/photocraft-web/.threaded-*" > /dev/null; then
  echo 'FAIL: temporary threaded entry point leaked' >&2
  exit 1
fi
if PATH="$FIXTURE/bin:$PATH" PHOTOCRAFT_TEST_NO_WORKER=1 bash "$FIXTURE/packaging/web/build-threaded.sh" > "$FIXTURE/worker-missing.log" 2>&1; then
  echo 'FAIL: a build without worker exports was accepted' >&2
  exit 1
fi
echo 'PASS: threaded entry point overrides Trunk HTML features and rejects missing worker exports'
