/**
 * The component gallery: every kit component with the mockups' own sample content, for review and
 * for the accessibility check in CI. A review tool, not part of the shipped app (it is built only
 * from gallery.html), so its sample text is English.
 */
import { useState } from "react";
import {
  Badge,
  Banner,
  Button,
  Checkbox,
  CodeDisplay,
  CodeInput,
  Dialog,
  MiniProgress,
  ProgressBar,
  ProgressValue,
  ResultChip,
  Row,
  RowList,
  Section,
  Segmented,
  Select,
  TextField,
  Toast,
  Toggle,
  type Result,
} from "../components/kit";
import { ArrowForward, Laptop, Lock } from "../icons";
import "./gallery.css";

const RESULTS: Result[] = ["copied", "reinstalled", "converted", "needs-action", "skipped", "failed"];

export default function Gallery() {
  const [code, setCode] = useState("");
  const [awake, setAwake] = useState(true);
  const [metered, setMetered] = useState(true);
  const [theme, setTheme] = useState<"light" | "system">("light");
  const [motion, setMotion] = useState<"system" | "reduced">("system");
  const [language, setLanguage] = useState("en");
  const [folders, setFolders] = useState({ documents: true, pictures: true, downloads: false });
  const [dialog, setDialog] = useState(false);
  const [toast, setToast] = useState(false);

  return (
    <main className="gallery">
      <h1>PCTwin component kit</h1>
      <p className="subhead">Every component with the mockups' sample content (design batch 01).</p>

      <Section title="Buttons">
        <div className="gallery-row">
          <Button variant="primary" icon={<ArrowForward />}>
            Start a move
          </Button>
          <Button>Restore setup</Button>
          <Button variant="citron">Open my new laptop</Button>
          <Button variant="quiet">Get a new code</Button>
          <Button disabled>Waiting</Button>
        </div>
      </Section>

      <Section title="Pairing code (C03)">
        <CodeDisplay
          digits="482716"
          meta={
            <>
              <Lock /> Expires in 2:00
            </>
          }
        />
        <div className="gallery-row gallery-spread">
          <span className="gallery-muted">Same Wi-Fi network</span>
          <Badge>Waiting for receiver</Badge>
        </div>
        <CodeInput value={code} onChange={setCode} label="Type the code shown on your old laptop" hint="Six digits. It changes every two minutes." />
      </Section>

      <Section title="Fields">
        <TextField label="Name for this laptop" placeholder="For example, Ada's laptop" hint="Shown on the other laptop while you pair." />
        <TextField label="Email address" defaultValue="ada@" error="Enter the whole email address, like name@example.com." />
      </Section>

      <Section title="Move progress (X01)">
        <div className="kit-progress-title">
          <div>
            <p className="eyebrow">Your move</p>
            <h2 className="gallery-heading">Settling you in.</h2>
          </div>
          <ProgressValue value={0.68} />
        </div>
        <ProgressBar value={0.68} label="Move progress" left="81.6 GB of 120 GB" right="About 18 min left" />
        <div className="gallery-space">
          <Banner tone="ready" title="Your essentials are ready.">
            Your documents, bookmarks and wallpaper are verified. You can start using your new laptop.
          </Banner>
        </div>
        <RowList>
          <Row icon={<Laptop />} title="Documents" end={<ResultChip result="copied" />} />
          <Row icon={<Laptop />} title="Apps" end="9 of 14 installed" />
          <Row icon={<Laptop />} title="Photos and videos" end="Copying" />
        </RowList>
      </Section>

      <Section title="Item results">
        <div className="gallery-row">
          {RESULTS.map((r) => (
            <ResultChip key={r} result={r} />
          ))}
        </div>
      </Section>

      <Section title="Settings (F10)">
        <Segmented
          label="Theme"
          hint="Choose how PCTwin looks."
          value={theme}
          options={[
            { value: "light", label: "Light" },
            { value: "system", label: "System" },
          ]}
          onChange={setTheme}
        />
        <Segmented
          label="Motion"
          hint="Use gentle transitions or reduce them."
          value={motion}
          options={[
            { value: "system", label: "System" },
            { value: "reduced", label: "Reduced" },
          ]}
          onChange={setMotion}
        />
        <Select
          label="Language"
          hint="Use your preferred language."
          value={language}
          options={[
            { value: "en", label: "English", lang: "en" },
            { value: "fr", label: "Français", lang: "fr" },
          ]}
          onChange={setLanguage}
        />
        <Toggle label="Keep this laptop awake" hint="Only while a move is running." checked={awake} onChange={setAwake} />
        <Toggle label="Ask on metered networks" hint="Before model or app downloads." checked={metered} onChange={setMetered} />
      </Section>

      <Section title="Choose what moves">
        <div className="gallery-card">
          <Checkbox
            label="Documents"
            hint="12,480 files"
            checked={folders.documents}
            onChange={(v) => {
              setFolders({ ...folders, documents: v });
            }}
          />
          <Checkbox
            label="Pictures"
            hint="8,214 files"
            checked={folders.pictures}
            onChange={(v) => {
              setFolders({ ...folders, pictures: v });
            }}
          />
          <Checkbox
            label="Downloads"
            hint="Not selected: you can add it later."
            checked={folders.downloads}
            onChange={(v) => {
              setFolders({ ...folders, downloads: v });
            }}
          />
        </div>
      </Section>

      <Section title="Banners">
        <Banner>Nothing moves until you review and approve the plan. Your old laptop stays unchanged during the move.</Banner>
        <div className="gallery-space">
          <Banner tone="attention">2 folders need permission</Banner>
        </div>
      </Section>

      <Section title="Mini progress (F09)">
        <div className="gallery-mini">
          <MiniProgress
            title="Your move, nearby."
            value={0.68}
            label="Move progress"
            left="Moving your selected files"
            right="About 18 min left"
            pause={<Button variant="quiet">Pause</Button>}
            open={<Button variant="primary">Open move</Button>}
          />
        </div>
      </Section>

      <Section title="Dialog and toast">
        <div className="gallery-row">
          <Button
            onClick={() => {
              setDialog(true);
            }}
          >
            Open a dialog
          </Button>
          <Button
            onClick={() => {
              setToast(true);
            }}
          >
            Show a toast
          </Button>
        </div>
        <Dialog
          open={dialog}
          title="Stop this move?"
          onClose={() => {
            setDialog(false);
          }}
          actions={
            <Button
              variant="primary"
              onClick={() => {
                setDialog(false);
              }}
            >
              Pause and keep what moved
            </Button>
          }
        >
          Everything already copied stays verified. You can continue later from the same point.
        </Dialog>
        {toast && (
          <Toast
            onDismiss={() => {
              setToast(false);
            }}
          >
            Move paused. Nothing was lost.
          </Toast>
        )}
      </Section>
    </main>
  );
}
