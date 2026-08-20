import init from "./eternalist.js";

const root = document.documentElement;
const status = document.getElementById("status");
const fail = error => {
  const message = error instanceof Error ? error.message : String(error);
  root.dataset.eternalist = "failed";
  status.textContent = "WEBGPU ATELIER FAILED · " + message;
};

try {
  if (!window.isSecureContext) {
    throw new Error("serve this directory over HTTPS or localhost");
  }
  if (!navigator.gpu) {
    throw new Error("this browser does not expose WebGPU");
  }
  const adapter = await navigator.gpu.requestAdapter({
    powerPreference: "high-performance",
  });
  if (!adapter) {
    throw new Error(
      "no usable WebGPU adapter; enable graphics acceleration. " +
      "In Linux Chromium, also enable chrome://flags/#enable-unsafe-webgpu " +
      "and chrome://flags/#enable-vulkan, then relaunch the browser",
    );
  }
  const info = adapter.info ?? {};
  const identity = [
    info.vendor,
    info.architecture,
    info.device,
    info.description,
    info.backend,
    info.type,
  ].filter(Boolean).join(" ").toLowerCase();
  const software = ["swiftshader", "llvmpipe", "lavapipe", "software", "cpu"]
    .find(marker => identity.includes(marker));
  root.dataset.webgpuAdapter = identity || "undisclosed";
  if (software) {
    throw new Error(
      `WebGPU selected a software adapter (${identity}); enable ` +
      "chrome://flags/#enable-vulkan, relaunch Chromium, and verify " +
      "chrome://gpu reports WebGPU as hardware accelerated",
    );
  }
  await init();
  window.setTimeout(() => {
    if (root.dataset.eternalist === "booting") {
      fail(new Error("the first rendered frame did not arrive"));
    }
  }, 15_000);
} catch (error) {
  fail(error);
}
