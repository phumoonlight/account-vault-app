export function FolderIcon({ open = false }: { open?: boolean }) {
  return (
    <svg
      className="folder-icon"
      width="16"
      height="16"
      viewBox="0 0 16 16"
      aria-hidden="true"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinejoin="round"
    >
      {open ? (
        <path d="M1.5 12.5v-9a1 1 0 0 1 1-1h3.6l1.6 1.6h5.3a1 1 0 0 1 1 1V6M1.5 12.5l1.9-5.6a1 1 0 0 1 1-.7h10.1l-2 6.3z" />
      ) : (
        <path d="M1.5 3.5a1 1 0 0 1 1-1h3.6l1.6 1.6h5.8a1 1 0 0 1 1 1v6.9a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z" />
      )}
    </svg>
  );
}
