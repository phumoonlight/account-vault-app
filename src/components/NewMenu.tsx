import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import type { EntryKind } from "../api";

const OPTIONS: { kind: EntryKind; label: string; hint: string }[] = [
  { kind: "account", label: "Account", hint: "Username, password, website" },
  { kind: "env", label: "Environment variables", hint: "KEY=value pairs, .env import" },
];

interface Props {
  onCreate: (kind: EntryKind) => void;
}

/** "New" button that opens a menu of entry kinds; Account is first and focused. */
export function NewMenu({ onCreate }: Props) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const items = useRef<(HTMLButtonElement | null)[]>([]);
  const trigger = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    items.current[0]?.focus();
    const onPointer = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onPointer);
    return () => document.removeEventListener("pointerdown", onPointer);
  }, [open]);

  function choose(kind: EntryKind) {
    setOpen(false);
    onCreate(kind);
  }

  function onMenuKey(e: KeyboardEvent) {
    const list = items.current.filter((b): b is HTMLButtonElement => !!b);
    const index = list.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "Escape" || e.key === "Tab") {
      e.preventDefault();
      setOpen(false);
      trigger.current?.focus();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      list[(index + step + list.length) % list.length]?.focus();
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      list[e.key === "Home" ? 0 : list.length - 1]?.focus();
    }
  }

  return (
    <div className="new-menu" ref={root}>
      <button
        ref={trigger}
        className="primary icon-button"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setOpen(true);
          }
        }}
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 14 14"
          aria-hidden="true"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
        >
          <path d="M7 2v10M2 7h10" />
        </svg>
        New
        <svg
          className="chevron"
          width="10"
          height="10"
          viewBox="0 0 10 10"
          aria-hidden="true"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M2 3.5 5 6.5 8 3.5" />
        </svg>
      </button>
      {open && (
        <div className="menu" role="menu" aria-label="Create" onKeyDown={onMenuKey}>
          {OPTIONS.map((o, i) => (
            <button
              key={o.kind}
              ref={(el) => {
                items.current[i] = el;
              }}
              role="menuitem"
              className="menu-item"
              onClick={() => choose(o.kind)}
            >
              <span className="menu-label">{o.label}</span>
              <span className="menu-hint">{o.hint}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
