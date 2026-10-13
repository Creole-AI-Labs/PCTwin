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

/**
 * The welcome picture, drawn to the mockup: the old laptop (white, tilted back 6 degrees) hands over
 * to the new one (blue, tilted forward 6 degrees) through the citron arrow tile.
 */
export function MoveIllustration({ className }: Props) {
  return (
    <svg className={className} viewBox="0 0 300 138" aria-hidden="true" focusable="false">
      <g transform="rotate(-6 80 66)">
        <rect x="9" y="21" width="142" height="90" rx="11" fill="#dde4f1" />
        <rect x="15" y="27" width="130" height="78" rx="6" fill="#ffffff" />
        <rect x="52" y="52" width="38" height="6" rx="3" fill="#a7baf0" />
        <rect x="52" y="63" width="55" height="6" rx="3" fill="#cbd8f6" />
        <rect x="52" y="74" width="55" height="6" rx="3" fill="#cbd8f6" />
        <rect x="-5" y="113" width="170" height="8" rx="4" fill="#dde4f1" />
      </g>
      <g transform="rotate(6 220 66)">
        <rect x="149" y="21" width="142" height="90" rx="11" fill="#173dc0" />
        <rect x="155" y="27" width="130" height="78" rx="6" fill="#2454f2" />
        <rect x="192" y="52" width="38" height="6" rx="3" fill="#e7f36a" />
        <rect x="192" y="63" width="55" height="6" rx="3" fill="#ffffff" fillOpacity="0.4" />
        <rect x="192" y="74" width="55" height="6" rx="3" fill="#ffffff" fillOpacity="0.4" />
        <rect x="135" y="113" width="170" height="8" rx="4" fill="#173dc0" />
      </g>
      <rect x="132" y="52" width="36" height="36" rx="12" fill="#e7f36a" />
      <path d="M141 70h18M152 63l7 7-7 7" stroke="#171b29" strokeWidth="1.8" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
