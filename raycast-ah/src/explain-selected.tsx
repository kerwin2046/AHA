import { Detail, getSelectedText, Clipboard, showToast, Toast, ActionPanel, Action } from "@raycast/api";
import { useEffect, useState } from "react";
import { explain, ExplainResult } from "./ah";

export default function Command() {
  const [state, setState] = useState<{
    loading: boolean;
    result?: ExplainResult & { raw: string };
    error?: string;
    source: string;
  }>({ loading: true, source: "", error: undefined });

  useEffect(() => {
    (async () => {
      try {
        let text = "";
        let source = "";

        try {
          text = await getSelectedText();
          if (text.trim()) source = "selection";
        } catch {
          text = (await Clipboard.readText()) || "";
          if (text.trim()) source = "clipboard";
        }

        if (!text?.trim()) {
          setState({
            loading: false,
            error: "No text selected or in clipboard.\n\nSelect text → Cmd+C → run this command again.",
            source: "",
          });
          return;
        }

        const sourceLabel = source === "selection" ? "selection" : "clipboard";
        setState({ loading: true, error: undefined, source: sourceLabel });
        const result = explain(text);
        setState({ loading: false, result, source: sourceLabel });
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        setState({ loading: false, error: msg, source: "" });
        await showToast({ title: "ah failed", message: msg, style: Toast.Style.Failure });
      }
    })();
  }, []);

  if (state.loading) {
    return <Detail markdown="Loading..." />;
  }

  if (state.error) {
    return (
      <Detail
        markdown={`# ah\n\n${state.error}`}
        actions={
          <ActionPanel>
            <Action.OpenInBrowser url="https://github.com/USER/ah" title="View ah Docs" />
          </ActionPanel>
        }
      />
    );
  }

  const r = state.result!;
  let md = `# ${r.translation || "Explanation"}`;
  if (r.explanation) md += `\n\n> ${r.explanation}`;
  if (r.usage) md += `\n\n---\n\n### Usage\n\n\`\`\`\n${r.usage}\n\`\`\``;
  md += `\n\n---\n_From ${state.source}_`;

  return (
    <Detail
      markdown={md}
      actions={
        <ActionPanel>
          <Action.CopyToClipboard content={r.raw} title="Copy Result" />
          <Action.CopyToClipboard content={r.translation} title="Copy Translation" />
        </ActionPanel>
      }
    />
  );
}
