import { SidebarSection } from "./SidebarSection";

interface ControlSidebarProps {
  collapsed: boolean;
  onToggle: () => void;
}

const SECTIONS = [
  { title: "Acquisition Control", icon: "▶" },
  { title: "Exposure & Gain", icon: "☀" },
  { title: "Image Format", icon: "⊞" },
  { title: "Trigger", icon: "⚡" },
] as const;

export function ControlSidebar({ collapsed, onToggle }: ControlSidebarProps) {
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
          {SECTIONS.map((section) => (
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
          {SECTIONS.map((section) => (
            <SidebarSection
              key={section.title}
              title={section.title}
              icon={section.icon}
            >
              <p className="sidebar-placeholder">
                Available in a future update.
              </p>
            </SidebarSection>
          ))}
        </>
      )}
    </aside>
  );
}
