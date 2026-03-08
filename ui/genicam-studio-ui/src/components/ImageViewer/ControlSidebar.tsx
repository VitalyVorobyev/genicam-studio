import type { ReactNode } from "react";
import type { EnumEntry, ParseXmlResponse } from "../../xml_model/uigraph";
import type { NodeValueEntry, SfncGroup } from "../../device/types";
import { useSfncGroups } from "../../device/useSfncGroups";
import { isSectionApplicable } from "./sfncGroupUtils";
import { SidebarSection } from "./SidebarSection";
import { AcquisitionSection } from "./AcquisitionSection";
import { ExposureGainSection } from "./ExposureGainSection";
import { ImageFormatSection } from "./ImageFormatSection";

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
  const groups = useSfncGroups();
  const nodesById = externalModel?.graph.nodes_by_name ?? {};
  const visibleGroups = groups.filter((g) => isSectionApplicable(g, nodesById));

  function renderSectionContent(group: SfncGroup): ReactNode {
    switch (group.id) {
      case "acquisition_control":
        return (
          <AcquisitionSection
            isConnected={isConnected}
            isAcquiring={isAcquiring}
            frameCount={frameCount}
            onStartAcq={onStartAcq}
            onStopAcq={onStopAcq}
            acquisitionModeEntries={acquisitionModeEntries}
          />
        );
      case "exposure_gain":
        return (
          <ExposureGainSection
            isConnected={isConnected}
            externalModel={externalModel}
            liveValues={liveValues}
          />
        );
      case "image_format":
        return (
          <ImageFormatSection
            isConnected={isConnected}
            externalModel={externalModel}
            liveValues={liveValues}
          />
        );
      default:
        return <p className="sidebar-placeholder">Available in a future update.</p>;
    }
  }

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
          {visibleGroups.map((group) => (
            <button
              key={group.id}
              type="button"
              className="iv-sidebar__rail-icon"
              title={group.title}
              onClick={onToggle}
            >
              {group.icon}
            </button>
          ))}
        </div>
      ) : (
        <>
          {groups.length === 0 ? (
            <p className="sidebar-placeholder">Loading controls…</p>
          ) : visibleGroups.length === 0 ? (
            <p className="sidebar-placeholder">No applicable sections for this device.</p>
          ) : (
            visibleGroups.map((group) => (
              <SidebarSection
                key={group.id}
                title={group.title}
                icon={group.icon}
                defaultOpen={group.default_open}
              >
                {renderSectionContent(group)}
              </SidebarSection>
            ))
          )}
        </>
      )}
    </aside>
  );
}
