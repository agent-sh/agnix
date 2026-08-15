#!/bin/bash
set -euo pipefail

# Download agnix binary for the current platform
# Environment variables:
#   AGNIX_VERSION - Version to download (default: latest)
#   BUILD_FROM_SOURCE - Set to "true" to build from source instead of downloading
#   GITHUB_TOKEN - Optional token for authenticated API requests (avoids rate limits)

REPO="agent-sh/agnix"
VERSION="${AGNIX_VERSION:-latest}"
BUILD_FROM_SOURCE="${BUILD_FROM_SOURCE:-false}"

# Create bin directory
BIN_DIR="${GITHUB_WORKSPACE:-$(pwd)}/.agnix-bin"
mkdir -p "${BIN_DIR}"

# Build from source if requested (useful for testing before releases exist)
if [ "${BUILD_FROM_SOURCE}" = "true" ]; then
    echo "Building agnix from source..."

    # Ensure Rust is available
    if ! command -v cargo &> /dev/null; then
        echo "Error: cargo not found. Install Rust to build from source." >&2
        exit 1
    fi

    # Build release binary
    cargo build --release -p agnix-cli --bin agnix

    # Copy to bin directory (handle both Unix and Windows binaries)
    if [ -f "target/release/agnix.exe" ]; then
        cp "target/release/agnix.exe" "${BIN_DIR}/"
        chmod +x "${BIN_DIR}/agnix.exe" 2>/dev/null || true
    elif [ -f "target/release/agnix" ]; then
        cp "target/release/agnix" "${BIN_DIR}/"
        chmod +x "${BIN_DIR}/agnix" 2>/dev/null || true
    else
        echo "Error: Could not find built binary" >&2
        exit 1
    fi
    echo "${BIN_DIR}" >> "${GITHUB_PATH:-/dev/null}"
    echo "agnix built from source and installed to ${BIN_DIR}"
    exit 0
fi

# Validate version format to prevent path traversal attacks (only for download path)
# Accepts: "latest" or semver like "v0.1.0", "v0.1.0-beta", "v0.1.0-beta-1+build"
# Use printf to avoid echo interpreting flags like -n or -e
if [ "${VERSION}" != "latest" ]; then
    if ! printf '%s' "${VERSION}" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$'; then
        echo "Error: Invalid version format: ${VERSION}" >&2
        echo "Expected: 'latest' or semver like 'v0.1.0'" >&2
        exit 1
    fi
fi

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

# Map to release artifact name
case "${OS}" in
    Linux)
        case "${ARCH}" in
            x86_64)
                TARGET="x86_64-unknown-linux-gnu"
                EXT="tar.gz"
                # Statically linked, so it still runs when the host glibc is
                # older than the one the gnu build was linked against (#1371).
                FALLBACK_TARGET="x86_64-unknown-linux-musl"
                ;;
            *)
                echo "Error: Unsupported Linux architecture: ${ARCH}" >&2
                exit 1
                ;;
        esac
        ;;
    Darwin)
        case "${ARCH}" in
            x86_64)
                TARGET="x86_64-apple-darwin"
                EXT="tar.gz"
                ;;
            arm64)
                TARGET="aarch64-apple-darwin"
                EXT="tar.gz"
                ;;
            *)
                echo "Error: Unsupported macOS architecture: ${ARCH}" >&2
                exit 1
                ;;
        esac
        ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
        TARGET="x86_64-pc-windows-msvc"
        EXT="zip"
        BINARY_NAME="agnix.exe"
        ;;
    *)
        echo "Error: Unsupported OS: ${OS}" >&2
        exit 1
        ;;
esac

# Set binary name (Windows uses .exe extension)
BINARY_NAME="${BINARY_NAME:-agnix}"
ARTIFACT_NAME="agnix-${TARGET}.${EXT}"
FALLBACK_TARGET="${FALLBACK_TARGET:-}"

# Resolve version
if [ "${VERSION}" = "latest" ]; then
    echo "Fetching latest release version..."
    # Use GITHUB_TOKEN if available to avoid rate limits
    CURL_OPTS=(-sL)
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        CURL_OPTS+=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
    fi
    # Use jq for robust JSON parsing (jq is a documented dependency)
    VERSION=$(curl "${CURL_OPTS[@]}" "https://api.github.com/repos/${REPO}/releases/latest" | jq -r '.tag_name // empty')
    if [ -z "${VERSION}" ]; then
        echo "Error: Could not determine latest version. No releases found." >&2
        echo "Please ensure a release exists at https://github.com/${REPO}/releases" >&2
        echo "Or set BUILD_FROM_SOURCE=true to build from source." >&2
        exit 1
    fi
fi

# Download and extract
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "${TEMP_DIR}"' EXIT

# install_artifact <artifact-name>
# Downloads one release artifact, verifies it against its checksum sidecar, and
# extracts it into BIN_DIR. Integrity failures exit non-zero: never install an
# artifact that failed verification.
install_artifact() {
    ARTIFACT_NAME="$1"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT_NAME}"

    echo "Downloading from ${DOWNLOAD_URL}..."
    HTTP_CODE=$(curl -sL -w "%{http_code}" "${DOWNLOAD_URL}" -o "${TEMP_DIR}/${ARTIFACT_NAME}")

    if [ "${HTTP_CODE}" != "200" ]; then
        echo "Error: Failed to download release (HTTP ${HTTP_CODE})" >&2
        echo "URL: ${DOWNLOAD_URL}" >&2
        exit 1
    fi

    CHECKSUM_URL="${DOWNLOAD_URL}.sha256"
    CHECKSUM_FILE="${TEMP_DIR}/${ARTIFACT_NAME}.sha256"

    echo "Downloading checksum from ${CHECKSUM_URL}..."
    HTTP_CODE=$(curl -sL -w "%{http_code}" "${CHECKSUM_URL}" -o "${CHECKSUM_FILE}")

    if [ "${HTTP_CODE}" != "200" ]; then
        echo "Error: Failed to download release checksum (HTTP ${HTTP_CODE})" >&2
        echo "URL: ${CHECKSUM_URL}" >&2
        exit 1
    fi

    EXPECTED_SHA="$(awk -v expected="${ARTIFACT_NAME}" '
    NF {
        hash = tolower($1)
        file = $2
        sub(/^\*/, "", file)
        sub(/\r$/, "", file)
        n = split(file, parts, /[\\\/]/)
        base = parts[n]
        if (base == expected) {
            print hash
            found = 1
            exit
        }
    }
    END {
        if (!found) {
            exit 1
        }
    }
    ' "${CHECKSUM_FILE}")" || {
        echo "Error: Checksum file does not contain an entry for ${ARTIFACT_NAME}" >&2
        exit 1
    }
    if ! printf '%s\n' "${EXPECTED_SHA}" | grep -Eq '^[0-9a-f]{64}$'; then
        echo "Error: Invalid checksum file for ${ARTIFACT_NAME}" >&2
        exit 1
    fi

    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL_SHA="$(sha256sum "${TEMP_DIR}/${ARTIFACT_NAME}" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')"
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL_SHA="$(shasum -a 256 "${TEMP_DIR}/${ARTIFACT_NAME}" | awk '{print $1}' | tr '[:upper:]' '[:lower:]')"
    else
        echo "Error: sha256sum or shasum is required to verify downloads" >&2
        exit 1
    fi

    if [ "${ACTUAL_SHA}" != "${EXPECTED_SHA}" ]; then
        echo "Error: Checksum mismatch for ${ARTIFACT_NAME}" >&2
        echo "Expected: ${EXPECTED_SHA}" >&2
        echo "Actual:   ${ACTUAL_SHA}" >&2
        exit 1
    fi

    echo "Checksum verified."

    echo "Extracting..."
    case "${ARTIFACT_NAME}" in
        *.tar.gz)
            tar -xzf "${TEMP_DIR}/${ARTIFACT_NAME}" -C "${BIN_DIR}"
            ;;
        *.zip)
            unzip -q -o "${TEMP_DIR}/${ARTIFACT_NAME}" -d "${BIN_DIR}"
            ;;
    esac

    # Make executable (use correct binary name for platform)
    chmod +x "${BIN_DIR}/${BINARY_NAME}" 2>/dev/null || true
}

echo "Downloading agnix ${VERSION} for ${TARGET}..."
install_artifact "agnix-${TARGET}.${EXT}"

# A binary built against a newer glibc than this host provides dies in the
# dynamic loader, so probe it and retry with the static build when one exists
# rather than leaving an unusable binary on PATH (#1371).
if ! "${BIN_DIR}/${BINARY_NAME}" --version >/dev/null 2>&1; then
    if [ -n "${FALLBACK_TARGET}" ]; then
        echo "Downloaded binary cannot run on this host, retrying with ${FALLBACK_TARGET}..."
        install_artifact "agnix-${FALLBACK_TARGET}.tar.gz"
        if ! "${BIN_DIR}/${BINARY_NAME}" --version >/dev/null 2>&1; then
            echo "Error: neither ${TARGET} nor ${FALLBACK_TARGET} runs on this host" >&2
            echo "Set BUILD_FROM_SOURCE=true to build from source instead." >&2
            exit 1
        fi
    else
        echo "Error: the downloaded ${TARGET} binary does not run on this host" >&2
        echo "Set BUILD_FROM_SOURCE=true to build from source instead." >&2
        exit 1
    fi
fi

# Add to PATH for subsequent steps
echo "${BIN_DIR}" >> "${GITHUB_PATH:-/dev/null}"

echo "agnix ${VERSION} installed to ${BIN_DIR}"
