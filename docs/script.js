const repository = "https://github.com/Atx85/potyi";
const releasePage = `${repository}/releases/latest`;
const apiUrl = "https://api.github.com/repos/Atx85/potyi/releases/latest";

const platformPatterns = {
  "macos-arm64": /(?:macos|darwin|osx).*(?:arm64|aarch64)/i,
  "macos-x86_64": /(?:macos|darwin|osx).*(?:x86_64|x64|intel)/i,
  windows: /(?:windows|win64|win-x64|\.msi$|\.exe$)/i,
  linux: /(?:linux|appimage|\.deb$)/i,
};

const releaseLabel = document.querySelector("#release-label");
const releaseNote = document.querySelector("#release-note");
const downloadRows = [...document.querySelectorAll("[data-platform]")];

function formatSize(bytes) {
  const megabytes = bytes / 1024 / 1024;
  return `${megabytes < 10 ? megabytes.toFixed(1) : Math.round(megabytes)} MB`;
}

async function connectLatestRelease() {
  try {
    const response = await fetch(apiUrl, {
      headers: { Accept: "application/vnd.github+json" },
    });

    if (!response.ok) {
      throw new Error(`GitHub returned ${response.status}`);
    }

    const release = await response.json();
    releaseLabel.textContent = release.name || release.tag_name || "Latest release";

    let linkedAssets = 0;

    for (const row of downloadRows) {
      const platform = row.dataset.platform;
      const asset = release.assets.find(({ name }) => platformPatterns[platform].test(name));
      const detail = row.querySelector("small");

      if (asset) {
        row.href = asset.browser_download_url;
        detail.textContent = `${asset.name} · ${formatSize(asset.size)}`;
        linkedAssets += 1;
      } else {
        row.href = release.html_url || releasePage;
        detail.textContent = "See release files";
      }
    }

    releaseNote.textContent = linkedAssets
      ? "Each button links to the matching file from the newest release."
      : "Open the latest release to see its available files.";
  } catch {
    releaseLabel.textContent = "Downloads";
    releaseNote.textContent = "Each button downloads the latest release for your platform.";
    releaseNote.dataset.state = "empty";
  }
}

connectLatestRelease();
