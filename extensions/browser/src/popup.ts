/**
 * Rocky Browser Extension — Popup script
 *
 * Handles the popup UI for adding resources (URLs) to Rocky topics.
 * Communicates with the background service worker and the Rocky sync API.
 */

document.addEventListener("DOMContentLoaded", async () => {
  const urlInput = document.getElementById("url") as HTMLInputElement;
  const titleInput = document.getElementById("title") as HTMLInputElement;
  const searchInput = document.getElementById("search") as HTMLInputElement;
  const topicList = document.getElementById("topicList") as HTMLDivElement;
  const addBtn = document.getElementById("addBtn") as HTMLButtonElement;
  const status = document.getElementById("status") as HTMLDivElement;

  const selectedTopics = new Set<string>();

  // Auto-populate URL and title from current tab
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (tab?.url) {
    urlInput.value = tab.url;
  }
  if (tab?.title) {
    titleInput.value = tab.title;
  }

  // Load topics from storage (synced from Rocky)
  const data = await chrome.storage.local.get("rockyTopics");
  const topics: string[] = data.rockyTopics ?? [];

  function renderTopics(filter: string): void {
    const filtered = filter
      ? topics.filter((t) =>
          t.toLowerCase().includes(filter.toLowerCase())
        )
      : topics;

    topicList.innerHTML = "";
    if (filtered.length === 0) {
      topicList.innerHTML =
        '<em style="font-size:12px; color:#6c7086;">No matching topics</em>';
      return;
    }

    for (const topic of filtered) {
      const chip = document.createElement("span");
      chip.className =
        "topic-chip" + (selectedTopics.has(topic) ? " selected" : "");
      chip.textContent = topic;
      chip.addEventListener("click", () => {
        if (selectedTopics.has(topic)) {
          selectedTopics.delete(topic);
          chip.classList.remove("selected");
        } else {
          selectedTopics.add(topic);
          chip.classList.add("selected");
        }
        addBtn.disabled = selectedTopics.size === 0;
      });
      topicList.appendChild(chip);
    }
  }

  renderTopics("");
  if (topics.length > 0) {
    addBtn.disabled = false;
  }

  searchInput.addEventListener("input", () => {
    renderTopics(searchInput.value);
  });

  addBtn.addEventListener("click", async () => {
    if (selectedTopics.size === 0) {
      return;
    }

    addBtn.disabled = true;
    status.textContent = "Adding…";
    status.className = "status";

    try {
      // Send to background for processing
      const response = await chrome.runtime.sendMessage({
        type: "ADD_RESOURCE",
        url: urlInput.value,
        title: titleInput.value,
        topics: [...selectedTopics],
      });

      if (response?.success) {
        status.textContent = "✓ Added to knowledge graph!";
        status.className = "status";
      } else {
        status.textContent = response?.error ?? "Failed to add resource.";
        status.className = "status error";
        addBtn.disabled = false;
      }
    } catch (err) {
      status.textContent = `Error: ${err}`;
      status.className = "status error";
      addBtn.disabled = false;
    }
  });
});
