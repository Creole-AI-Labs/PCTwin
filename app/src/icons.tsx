/** Small line icons, drawn here so nothing is fetched. All are decorative: the words carry meaning. */

type Props = { className?: string };

const base = {
  width: 20,
  height: 20,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
  focusable: false,
};

export function ArrowForward({ className }: Props) {
  return (
    <svg {...base} className={`flip ${className ?? ""}`}>
      <path d="M5 12h14M13 6l6 6-6 6" />
    </svg>
  );
}

export function ArrowBack({ className }: Props) {
  return (
    <svg {...base} className={`flip ${className ?? ""}`}>
      <path d="M15 6l-6 6 6 6" />
    </svg>
  );
}

export function Laptop({ className }: Props) {
  return (
    <svg {...base} width={28} height={28} className={className}>
      <rect x="5" y="5" width="14" height="10" rx="1.5" />
      <path d="M3 19h18" />
    </svg>
  );
}

export function Info({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5M12 8h.01" />
    </svg>
  );
}

export function Lock({ className }: Props) {
  return (
    <svg {...base} width={16} height={16} className={className}>
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V8a4 4 0 0 1 8 0v3" />
    </svg>
  );
}

export function History({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M3 12a9 9 0 1 0 3-6.7L3 8" />
      <path d="M3 3v5h5M12 7v5l3 3" />
    </svg>
  );
}

export function Wrench({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M14.7 6.3a4 4 0 0 0 5 5l-8.4 8.4a2.1 2.1 0 0 1-3-3l8.4-8.4a4 4 0 0 0-2-2z" />
    </svg>
  );
}

/** The welcome picture: the old laptop handing over to the new one. */
export function MoveIllustration({ className }: Props) {
  return (
    <svg className={className} viewBox="0 0 320 120" aria-hidden="true" focusable="false">
      <g transform="rotate(-3 80 60)">
        <rect x="18" y="16" width="118" height="78" rx="10" fill="#dfe6f3" />
        <rect x="26" y="24" width="102" height="62" rx="5" fill="#ffffff" />
        <rect x="42" y="38" width="38" height="6" rx="3" fill="#a9b9ef" />
        <rect x="42" y="52" width="62" height="6" rx="3" fill="#c9d4f6" />
        <rect x="42" y="66" width="56" height="6" rx="3" fill="#c9d4f6" />
        <rect x="8" y="96" width="140" height="8" rx="4" fill="#dfe6f3" />
      </g>
      <g transform="rotate(4 240 60)">
        <rect x="182" y="12" width="122" height="82" rx="10" fill="#1a3fd0" />
        <rect x="190" y="20" width="106" height="66" rx="5" fill="var(--colour-blue)" />
        <rect x="206" y="34" width="40" height="6" rx="3" fill="var(--colour-citron)" />
        <rect x="206" y="48" width="66" height="6" rx="3" fill="#7d97f7" />
        <rect x="206" y="62" width="60" height="6" rx="3" fill="#7d97f7" />
        <rect x="174" y="96" width="140" height="8" rx="4" fill="#1a3fd0" />
      </g>
      <rect x="138" y="46" width="40" height="40" rx="11" fill="var(--colour-citron)" />
      <path d="M149 66h18M161 59l7 7-7 7" stroke="#171b29" strokeWidth="2.4" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
