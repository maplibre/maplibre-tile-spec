let panzoomScrollPosition = 0;

// Constants for localStorage state management
const THIRTY_DAYS_IN_MS = 30 * 24 * 60 * 60 * 1000; // 30 days in milliseconds
const SAVE_DEBOUNCE_DELAY_MS = 200; // Debounce delay for saving zoom state in milliseconds

// Constants for zoom functionality
const DEFAULT_ZOOM_LEVEL = 1.0; // Default browser zoom level (100%)

// LocalStorage utility functions for saving zoom levels
function getStorageKey(boxId) {
  const pageUrl = window.location.pathname;
  return `panzoom-${pageUrl}-${boxId}`;
}

function saveZoomState(boxId, transform) {
  console.log(boxId);
  try {
    const key = getStorageKey(boxId);
    localStorage.setItem(
      key,
      JSON.stringify({
        x: transform.x,
        y: transform.y,
        scale: transform.scale,
        timestamp: Date.now(),
      })
    );
  } catch (e) {
    console.warn("Failed to save zoom state to localStorage:", e);
  }
}

function loadZoomState(boxId) {
  try {
    const key = getStorageKey(boxId);
    const saved = localStorage.getItem(key);
    if (saved) {
      const state = JSON.parse(saved);
      // Only use saved state if it's less than 30 days old
      if (Date.now() - state.timestamp < THIRTY_DAYS_IN_MS) {
        return state;
      }
    }
  } catch (e) {
    console.warn("Failed to load zoom state from localStorage:", e);
  }
  return null;
}

function clearZoomState(boxId) {
  try {
    const key = getStorageKey(boxId);
    localStorage.removeItem(key);
  } catch (e) {
    console.warn("Failed to clear zoom state from localStorage:", e);
  }
}

function minimize(instance, box, max, min) {
  box.classList.remove("panzoom-fullscreen");
  max.classList.remove("panzoom-hidden");
  min.classList.add("panzoom-hidden");
  panzoom_reset(instance, box);
  setTimeout(() => {
    window.scrollTo(0, panzoomScrollPosition);
  }, 0);
}

function maximize(instance, box, max, min) {
  panzoomScrollPosition =
    window.pageYOffset || document.documentElement.scrollTop;

  box.classList.add("panzoom-fullscreen");
  max.classList.add("panzoom-hidden");
  min.classList.remove("panzoom-hidden");
}

function escapeFullScreen(e, box, max, min, instance) {
  if (e.keyCode == 27) {
    minimize(instance, box, max, min);
  }
}

function panzoom_reset(instance, box) {
  // Clear saved zoom state when resetting
  if (box && box.id) {
    clearZoomState(box.id);
  }

  // Reset to initial position and default browser zoom level
  instance.moveTo(0, 0);
  instance.zoomAbs(0, 0, DEFAULT_ZOOM_LEVEL);
}

function add_buttons(box, instance) {
  let reset = box.querySelector(".panzoom-reset");
  let max = box.querySelector(".panzoom-max");
  let min = box.querySelector(".panzoom-min");
  let info = box.querySelector(".panzoom-info");
  let info_box = box.querySelector(".panzoom-info-box");
  let nav = box.querySelector(".panzoom-nav");

  reset.addEventListener("click", function (e) {
    panzoom_reset(instance, box); // Always reset to default browser zoom level
  });
  if (info != undefined) {
    info.addEventListener("click", function (e) {
      if (box.dataset.info == "true") {
        box.dataset.info = false;
        info_box.classList.add("panzoom-hidden");

        if (nav.classList.contains("panzoom-top-nav")) {
          nav.classList.remove("panzoom-nav-infobox-top");
        }
      } else {
        box.dataset.info = true;
        info_box.classList.remove("panzoom-hidden");
        if (nav.classList.contains("panzoom-top-nav")) {
          nav.classList.add("panzoom-nav-infobox-top");
        }
      }
    });
  }
  if (max != undefined) {
    max.addEventListener("click", function (e) {
      maximize(instance, box, max, min);
    });
  }
  if (min != undefined) {
    min.addEventListener("click", function (e) {
      minimize(instance, box, max, min); // Always reset to default browser zoom level
    });
  }
  box.addEventListener("keydown", function (e) {
    escapeFullScreen(e, box, max, min, instance); // Always reset to default browser zoom level
  });
}

function activate_zoom_pan() {
  let boxes = document.querySelectorAll(".panzoom-box");

  let meta_tag = document.querySelector('meta[name="panzoom-data"]');

  let panzoomData = {};
  let selectors = [".panzoom-content"]; // Default selector
  let initialZoomLevel = DEFAULT_ZOOM_LEVEL; // Default zoom level

  try {
    panzoomData = JSON.parse(meta_tag.content);
    selectors = panzoomData.selectors || [];
    initialZoomLevel = panzoomData.initial_zoom_level ?? DEFAULT_ZOOM_LEVEL;
  } catch (e) {
    console.warn("Failed to parse panzoom data:", e);
  }

  let box_counter = 0;

  boxes.forEach((box) => {
    let key = box.dataset.key;
    let elem;

    let boxID = box_counter;
    box_counter += 1;

    selectors.every((selector) => {
      elem = box.querySelector(selector);

      if (elem != undefined) {
        return false;
      }
      return true;
    });

    if (elem == undefined) {
      return;
    }

    if (
      (elem.nodeName == "DIV" || elem.nodeName == "IMG") &&
      !elem.dataset.zoom
    ) {
      elem.dataset.zoom = true;

      // Create panzoom instance
      let instance = panzoom(elem, {
        minZoom: 0.5,
        beforeWheel: function (e) {
          switch (key) {
            case "ctrl":
              return !e.ctrlKey;
            case "shift":
              return !e.shiftKey;
            case "alt":
              return !e.altKey;
            default:
              return false && !e.button == 1;
          }
        },
        beforeMouseDown: function (e) {
          switch (key) {
            case "ctrl":
              return !e.ctrlKey && !e.button == 1;
            case "shift":
              return !e.shiftKey && !e.button == 1;
            case "alt":
              return !e.altKey && !e.button == 1;
            default:
              return false && !e.button == 1;
          }
        },
        zoomDoubleClickSpeed: 1,
      });

      // Load saved zoom state or use initial zoom level
      const savedState = loadZoomState(boxID);
      if (savedState) {
        // Apply saved zoom state
        instance.zoomAbs(0, 0, savedState.scale);
        instance.moveTo(savedState.x, savedState.y);
      } else if (initialZoomLevel !== DEFAULT_ZOOM_LEVEL) {
        // Apply configured initial zoom level
        instance.zoomAbs(0, 0, initialZoomLevel);
      }

      // Save zoom state when it changes
      let zoomSaveTimeout;
      let panSaveTimeout;
      instance.on("zoom", function () {
        // Debounce saving to avoid excessive localStorage writes
        clearTimeout(zoomSaveTimeout);
        zoomSaveTimeout = setTimeout(() => {
          const transform = instance.getTransform();
          saveZoomState(boxID, transform);
        }, SAVE_DEBOUNCE_DELAY_MS);
      });
      instance.on("pan", function () {
        // Debounce saving to avoid excessive localStorage writes
        clearTimeout(panSaveTimeout);
        panSaveTimeout = setTimeout(() => {
          const transform = instance.getTransform();
          saveZoomState(boxID, transform);
        }, SAVE_DEBOUNCE_DELAY_MS);
      });

      add_buttons(box, instance);
    }
  });
}

// handle themes differently
let pz_theme = document.querySelector('meta[name="panzoom-theme"]').content;

if (pz_theme == "material") {
  document$.subscribe(function () {
    const interval = setInterval(activate_zoom_pan, 1000);

    setTimeout(function () {
      clearInterval(interval);
    }, 5000);
  });
} else {
  const interval = setInterval(activate_zoom_pan, 1000);

  setTimeout(function () {
    clearInterval(interval);
  }, 5000);
}
