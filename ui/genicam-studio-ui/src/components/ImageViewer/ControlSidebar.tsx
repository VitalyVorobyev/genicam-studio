import type { EnumEntry, ParseXmlResponse } from "../../xml_model/uigraph";
import type { NodeValueEntry } from "../../device/types";
import { SidebarSection } from "./SidebarSection";
import { AcquisitionSection } from "./AcquisitionSection";
import { ExposureGainSection } from "./ExposureGainSection";

interface ControlSidebarProps {
  collapsed: boolean;
  onToggle: () => void;
  isConnected: boolean;
  isAcquiring: boolean;
  frameCount: number;
  onStartAcq: () => Promise<void>;
  onStopAcq: () => Promise<void>;
  acquisitionModeEntries: EnumEntry[];
  liveValues: Map<string, NodeValueEntry>;
  externalModel: ParseXmlResponse | null;
}

const RAIL_SECTIONS = [
  { title: "Acquisition Control", icon: "▶" },
  { title: "Exposure & Gain", icon: "☀" },
  { title: "Image Format", icon: "⊞" },
  { title: "Trigger", icon: "⚡" },
] as const;

export function ControlSidebar({
  collapsed,
  onToggle,
  isConnected,
  isAcquiring,
  frameCount,
  onStartAcq,
  onStopAcq,
  acquisitionModeEntries,
  liveValues,
  externalModel,
}: ControlSidebarProps) {
  return (
    <aside
      className={`iv-sidebar${collapsed ? " iv-sidebar--collapsed" : ""}`}
      aria-label="Control sidebar"
    >
      <button
        type="button"
        className="iv-sidebar__toggle"
        aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
        onClick={onToggle}
      >
        {collapsed ? "›" : "‹"}
      </button>

      {collapsed ? (
        <div className="iv-sidebar__rail">
          {RAIL_SECTIONS.map((section) => (
            <button
              key={section.title}
              type="button"
              className="iv-sidebar__rail-icon"
              aria-label={section.title}
              onClick={onToggle}
              title={section.title}
            >
              {section.icon}
            </button>
          ))}
        </div>
      ) : (
        <>
          <SidebarSection title="Acquisition Control" icon="▶">
            <AcquisitionSection
              isConnected={isConnected}
              isAcquiring={isAcquiring}
              frameCount={frameCount}
              onStartAcq={onStartAcq}
              onStopAcq={onStopAcq}
              acquisitionModeEntries={acquisitionModeEntries}
            />
          </SidebarSection>

          <SidebarSection title="Exposure & Gain" icon="☀">
            <ExposureGainSection
              isConnected={isConnected}
              externalModel={externalModel}
              liveValues={liveValues}
            />
          </SidebarSection>

          <SidebarSection title="Image Format" icon="⊞">
            <p className="sidebar-placeholder">Available in a future update.</p>
          </SidebarSection>

          <SidebarSection title="Trigger" icon="⚡">
            <p className="sidebar-placeholder">Available in a future update.</p>
          </SidebarSection>
        </>
      )}
    </aside>
  );
}
