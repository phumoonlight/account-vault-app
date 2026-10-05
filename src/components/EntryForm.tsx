import { useState, type FormEvent } from "react";
import { emptyInput, type CustomField, type Entry, type EntryInput } from "../api";
import { generatePassword } from "../password";

interface Props {
  /** Entry being edited, or undefined when creating a new one. */
  initial?: Entry;
  onSave: (input: EntryInput) => Promise<void>;
  onCancel: () => void;
}

export function EntryForm({ initial, onSave, onCancel }: Props) {
  const [form, setForm] = useState<EntryInput>(() =>
    initial
      ? {
          title: initial.title,
          username: initial.username,
          password: initial.password,
          url: initial.url,
          notes: initial.notes,
          customFields: initial.customFields.map((f) => ({ ...f })),
          favorite: initial.favorite,
        }
      : emptyInput(),
  );
  const [showPassword, setShowPassword] = useState(false);
  const [saving, setSaving] = useState(false);

  const set = <K extends keyof EntryInput>(key: K, value: EntryInput[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  const setField = (index: number, patch: Partial<CustomField>) =>
    set(
      "customFields",
      form.customFields.map((f, i) => (i === index ? { ...f, ...patch } : f)),
    );

  async function submit(e: FormEvent) {
    e.preventDefault();
    setSaving(true);
    try {
      await onSave({
        ...form,
        title: form.title.trim(),
        customFields: form.customFields.filter((f) => f.name.trim() || f.value),
      });
    } finally {
      setSaving(false);
    }
  }

  return (
    <form className="pane" onSubmit={submit}>
      <header className="pane-header">
        <h2>{initial ? "Edit entry" : "New entry"}</h2>
        <div className="actions">
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={saving || !form.title.trim()}>
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </header>

      <div className="form-grid">
        <label>
          Title <span className="required">*</span>
          <input
            autoFocus
            value={form.title}
            onChange={(e) => set("title", e.target.value)}
            placeholder="e.g. GitHub"
          />
        </label>
        <label>
          Username
          <input
            value={form.username}
            onChange={(e) => set("username", e.target.value)}
            placeholder="Username or email"
            autoComplete="off"
          />
        </label>
        <label>
          Password
          <div className="input-row">
            <input
              type={showPassword ? "text" : "password"}
              value={form.password}
              onChange={(e) => set("password", e.target.value)}
              autoComplete="new-password"
              spellCheck={false}
            />
            <button type="button" onClick={() => setShowPassword((s) => !s)}>
              {showPassword ? "Hide" : "Show"}
            </button>
            <button
              type="button"
              onClick={() => {
                set("password", generatePassword());
                setShowPassword(true);
              }}
            >
              Generate
            </button>
          </div>
        </label>
        <label>
          Website
          <input
            value={form.url}
            onChange={(e) => set("url", e.target.value)}
            placeholder="https://"
          />
        </label>
        <label>
          Notes
          <textarea rows={4} value={form.notes} onChange={(e) => set("notes", e.target.value)} />
        </label>

        <fieldset>
          <legend>Custom fields</legend>
          {form.customFields.map((f, i) => (
            <div className="custom-field" key={i}>
              <input
                placeholder="Name (e.g. PIN)"
                value={f.name}
                onChange={(e) => setField(i, { name: e.target.value })}
              />
              <input
                placeholder="Value"
                type={f.secret ? "password" : "text"}
                value={f.value}
                onChange={(e) => setField(i, { value: e.target.value })}
                autoComplete="off"
              />
              <label className="checkbox">
                <input
                  type="checkbox"
                  checked={f.secret}
                  onChange={(e) => setField(i, { secret: e.target.checked })}
                />
                Hidden
              </label>
              <button
                type="button"
                className="small"
                aria-label="Remove field"
                onClick={() =>
                  set(
                    "customFields",
                    form.customFields.filter((_, j) => j !== i),
                  )
                }
              >
                Remove
              </button>
            </div>
          ))}
          <button
            type="button"
            className="small"
            onClick={() =>
              set("customFields", [...form.customFields, { name: "", value: "", secret: false }])
            }
          >
            + Add field
          </button>
        </fieldset>

        <label className="checkbox">
          <input
            type="checkbox"
            checked={form.favorite}
            onChange={(e) => set("favorite", e.target.checked)}
          />
          Favorite
        </label>
      </div>
    </form>
  );
}
