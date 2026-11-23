import { Cpu, Gauge, HardDrive, Sparkles, Thermometer } from "lucide-react";
import { formatVramGb, type HardwareProfile } from "../../lib/ipc";
import { useEngineStore } from "../../stores/useEngineStore";

interface HardwareBannerProps {
  hardware: HardwareProfile | null;
  scanning: boolean;
}

export function HardwareBanner({ hardware, scanning }: HardwareBannerProps) {
  const telemetry = useEngineStore((s) => s.telemetry);

  if (!hardware) {
    return (
      <section className="rounded-xl border border-border-default bg-surface-card p-5">
        <div className="flex items-center gap-3 text-sm text-text-secondary">
          <Cpu className="h-5 w-5 text-primary animate-pulse" />
          <span>
            {scanning
              ? "Scanning DXGI GPU adapters, Vulkan runtime, and system RAM..."
              : "Waiting for Windows hardware scan results..."}
          </span>
        </div>
      </section>
    );
  }

  const tierInfo = (() => {
    switch (hardware.recommended_tier) {
      case "optimal_8b":
        return {
          pill: "Optimal Tier (≥ 8 GB VRAM)",
          pillClass: "border-primary-container/30 bg-primary-container/12 text-primary",
          explanation:
            "Your GPU has ≥ 7.5 GB Dedicated VRAM with Vulkan support. Qwen 3 8B (Q4_K_M) fits completely in VRAM for the highest quality in idioms, literature, and film/anime dialogue.",
        };
      case "balanced_4b":
        return {
          pill: "Balanced Tier (6 GB VRAM)",
          pillClass: "border-tag-optimize/30 bg-tag-optimize/12 text-tag-optimize",
          explanation:
            "Your GPU is in the 6 GB VRAM class. Qwen 3 4B (Q4_K_M) is recommended because it fits comfortably in VRAM (~3.6 GB) with very fast token throughput and zero RAM spillover.",
        };
      default:
        return {
          pill: hardware.vulkan_supported ? "Fallback Tier (< 5.5 GB VRAM)" : "Fallback CPU Mode",
          pillClass: "border-tag-optimize/30 bg-tag-optimize/12 text-tag-optimize",
          explanation: hardware.vulkan_supported
            ? "Available VRAM is below 5.5 GB. Osmium recommends Qwen 3 4B (Q4_K_M) with automatic partial GPU offload to keep responsiveness high."
            : "Active Vulkan support was not detected on your GPU. Osmium recommends Qwen 3 4B (Q4_K_M) running steadily in CPU mode.",
        };
    }
  })();

  const hasThermals = hardware.thermal_c != null || hardware.clock_mhz != null;

  return (
    <section className="rounded-xl border border-border-default bg-surface-card p-5 space-y-4">
      {/* Header Row */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2.5 min-w-0">
          <h2 className="text-lg font-semibold leading-7 text-text-primary">
            {hardware.gpu_name}
          </h2>
          <span
            className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium ${tierInfo.pillClass}`}
          >
            {tierInfo.pill}
          </span>
        </div>

        <div className="flex flex-wrap items-center gap-2 text-xs font-medium">
          <span
            className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 ${
              hardware.vulkan_supported
                ? "border-primary-container/30 bg-primary-container/10 text-primary"
                : "border-tag-optimize/30 bg-tag-optimize/10 text-tag-optimize"
            }`}
          >
            <span
              className={`h-1.5 w-1.5 rounded-full ${
                hardware.vulkan_supported ? "bg-primary" : "bg-tag-optimize"
              }`}
            />
            {hardware.vulkan_supported
              ? `Vulkan ${hardware.vulkan_version ?? "1.x"} Native${
                  hardware.vulkan_device_id ? ` (${hardware.vulkan_device_id})` : ""
                }`
              : "CPU Mode"}
          </span>

          <span className="rounded-full border border-border-default bg-surface-container px-2.5 py-1 text-text-secondary tabular-nums">
            System RAM: {hardware.system_ram_gb.toFixed(1)} GB
          </span>
        </div>
      </div>

      {/* 4 Diagnostic Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="flex items-center justify-between gap-2 text-xs font-medium text-text-secondary">
            <span>Dedicated VRAM Total</span>
            <HardDrive className="h-4 w-4 text-primary" />
          </div>
          <div className="mt-2 text-xl font-semibold leading-7 text-text-primary tabular-nums">
            {hardware.dedicated_vram_mb.toLocaleString("en-US")} MB
          </div>
          <div className="mt-1 text-xs leading-4 text-text-muted tabular-nums">
            {formatVramGb(hardware.dedicated_vram_mb)} • DXGI DedicatedVideoMemory
          </div>
        </div>

        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="flex items-center justify-between gap-2 text-xs font-medium text-text-secondary">
            <span>Kernel Accelerators</span>
            <Sparkles className="h-4 w-4 text-tag-security" />
          </div>
          <div className="mt-2 text-base font-semibold leading-6 text-text-primary">
            {hardware.flash_attention_supported ? "Flash Attention + Q8_0" : "CPU Fallback"}
          </div>
          <div className="mt-1 text-xs leading-4 text-text-muted">
            {hardware.flash_attention_supported
              ? "KV Cache Q8_0 Supported"
              : "GPU acceleration inactive"}
          </div>
        </div>

        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="flex items-center justify-between gap-2 text-xs font-medium text-text-secondary">
            <span>Shared Host Memory</span>
            <Gauge className="h-4 w-4 text-tag-organize" />
          </div>
          <div className="mt-2 text-xl font-semibold leading-7 text-text-primary tabular-nums">
            {hardware.shared_memory_mb.toLocaleString("en-US")} MB
          </div>
          <div className="mt-1 text-xs leading-4 text-text-muted">
            DXGI SharedSystemMemory
          </div>
        </div>

        <div className="rounded-lg border border-border-default bg-surface-elevated p-3.5">
          <div className="flex items-center justify-between gap-2 text-xs font-medium text-text-secondary">
            <span>{hasThermals ? "Thermals & Clock" : "Engine & Offload Status"}</span>
            <Thermometer className="h-4 w-4 text-tag-optimize" />
          </div>
          {hasThermals ? (
            <>
              <div className="mt-2 text-xl font-semibold leading-7 text-text-primary tabular-nums">
                {hardware.thermal_c != null ? `${hardware.thermal_c}°C` : "--"}{" "}
                {hardware.clock_mhz != null ? `• ${hardware.clock_mhz} MHz` : ""}
              </div>
              <div className="mt-1 text-xs leading-4 text-text-muted">
                Real-Time NVML GPU Sensor
              </div>
            </>
          ) : (
            <>
              <div className="mt-2 text-base font-semibold leading-6 text-text-primary">
                {hardware.vulkan_supported
                  ? `${telemetry?.gpu_layers ?? 99} GPU Layers Ready`
                  : "CPU Offload Only"}
              </div>
              <div className="mt-1 text-xs leading-4 text-text-muted">
                Direct Memory Mapped (Vendor-Agnostic)
              </div>
            </>
          )}
        </div>
      </div>

      {/* Human-Friendly Recommendation Box */}
      <div className="rounded-lg border border-border-subtle bg-surface-container px-4 py-3 text-sm leading-5 text-text-secondary">
        <span className="font-medium text-text-primary">Hardware Recommendation: </span>
        {tierInfo.explanation}
      </div>
    </section>
  );
}
