import { useId, useState, type KeyboardEvent } from "react";
import { addTag, MAX_TAGS, sameTag } from "../tags";

interface Props {
  /** id for the text input, so a <label htmlFor> targets it (not the first chip's remove button). */
  id?: string;
  value: string[];
  onChange: (tags: string[]) => void;
  /** Existing tags offered as autocomplete. */
  suggestions: string[];
}

/** Tags as removable chips. Enter or comma adds; Backspace on empty input removes the last. */
export function TagInput({ id, value, onChange, suggestions }: Props) {
  const [text, setText] = useState("");
  const listId = useId();
  const full = value.length >= MAX_TAGS;

  function commit() {
    if (text.trim()) onChange(addTag(value, text));
    setText("");
  }

  /** Typing or pasting "a, b, c" adds "a" and "b" as tags and keeps "c" as text. */
  function onTextChange(next: string) {
    const parts = next.split(",");
    if (parts.length > 1) {
      onChange(parts.slice(0, -1).reduce(addTag, value));
    }
    setText(parts[parts.length - 1]);
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter" && text.trim()) {
      e.preventDefault(); // Don't submit the form.
      commit();
    } else if (e.key === "Backspace" && !text && value.length) {
      onChange(value.slice(0, -1));
    }
  }

  return (
    <div className="tag-input">
      {value.map((tag) => (
        <span className="tag" key={tag}>
          {tag}
          <button
            type="button"
            className="tag-remove"
            aria-label={`Remove tag ${tag}`}
            onClick={() => onChange(value.filter((t) => t !== tag))}
          >
            ×
          </button>
        </span>
      ))}
      <input
        id={id}
        list={listId}
        value={text}
        placeholder={full ? `Max ${MAX_TAGS} tags` : value.length ? "Add tag" : "e.g. work, bank"}
        disabled={full}
        onChange={(e) => onTextChange(e.target.value)}
        onKeyDown={onKeyDown}
        onBlur={commit}
      />
      <datalist id={listId}>
        {suggestions
          .filter((s) => !value.some((t) => sameTag(t, s)))
          .map((s) => (
            <option key={s} value={s} />
          ))}
      </datalist>
    </div>
  );
}
