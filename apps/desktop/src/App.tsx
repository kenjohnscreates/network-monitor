import { useState } from "react";
import InterfaceList from "./InterfaceList";
import CapturePanel from "./CapturePanel";
import ConnectionsTable from "./ConnectionsTable";
import Dashboard from "./Dashboard";
import TrafficHealthStrip from "./TrafficHealthStrip";
import SignalsPanel from "./SignalsPanel";
import SettingsModal from "./SettingsPanel";
import ExportButtons from "./ExportButtons";
import { useFlowSnapshot } from "./useFlowSnapshot";

export default function App() {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [highlightFlowId, setHighlightFlowId] = useState<string | null>(null);
  const { snapshot, history } = useFlowSnapshot();

  return (
    <main className="app">
      <header className="app-head">
        <div>
          <h1>
            &gt; Network Monitor<span className="cursor">_</span>
          </h1>
          <p className="subtitle">// personal-use desktop traffic monitor</p>
        </div>
        <div className="row gap">
          <ExportButtons />
          <button
            className="secondary"
            aria-label="settings"
            onClick={() => setSettingsOpen(true)}
          >
            settings
          </button>
        </div>
      </header>

      <InterfaceList selectedId={selectedId} onSelect={setSelectedId} />

      <CapturePanel interfaceId={selectedId} />

      <TrafficHealthStrip snapshot={snapshot} />

      <Dashboard snapshot={snapshot} history={history} />

      <SignalsPanel
        snapshot={snapshot}
        onHighlightFlow={(id) => setHighlightFlowId(id)}
      />

      <ConnectionsTable
        snapshot={snapshot}
        highlightFlowId={highlightFlowId}
      />

      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}

      <footer>
        <small>v1 — capture, aggregate, dashboard, filter, enrich, export.</small>
      </footer>
    </main>
  );
}
