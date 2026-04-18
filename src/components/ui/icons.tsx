import type { SVGProps } from 'react';

export type IconName =
  | 'overview'
  | 'topics'
  | 'groups'
  | 'messages'
  | 'replay'
  | 'trace'
  | 'saved'
  | 'audit'
  | 'settings'
  | 'search'
  | 'recent'
  | 'user'
  | 'cluster';

type IconProps = SVGProps<SVGSVGElement> & {
  name: IconName;
};

const iconPaths: Record<IconName, JSX.Element> = {
  overview: <path d="M4 5h6v6H4zM14 5h6v10h-6zM4 15h6v4H4zM14 17h6v2h-6z" />,
  topics: <path d="M4 7h16M4 12h16M4 17h10M6 5v14" />,
  groups: <path d="M8 9a2.5 2.5 0 1 0 0-.01zM16.5 8a2 2 0 1 0 0-.01zM4.5 18a3.5 3.5 0 0 1 7 0M13 18a3 3 0 0 1 6 0" />,
  messages: <path d="M5 6h14v10H9l-4 3zM8 10h8M8 13h5" />,
  replay: <path d="M7 7H4v6M4 13a8 8 0 1 0 2-5.3" />,
  trace: <path d="M5 6h4v4H5zM15 6h4v4h-4zM10 14h4v4h-4zM9 8h6M7 10l5 4M17 10l-5 4" />,
  saved: <path d="M6 4h12v16l-6-4-6 4z" />,
  audit: <path d="M12 4l7 3v5c0 4.5-2.7 7-7 8-4.3-1-7-3.5-7-8V7l7-3zM9 12l2 2 4-4" />,
  settings: <path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7zm8 3.5-.9-.4a7.4 7.4 0 0 0-.5-1.2l.5-.9-1.8-1.8-.9.5c-.4-.2-.8-.4-1.2-.5l-.4-.9h-2.6l-.4.9c-.4.1-.8.3-1.2.5l-.9-.5-1.8 1.8.5.9c-.2.4-.4.8-.5 1.2L4 12v2.6l.9.4c.1.4.3.8.5 1.2l-.5.9 1.8 1.8.9-.5c.4.2.8.4 1.2.5l.4.9h2.6l.4-.9c.4-.1.8-.3 1.2-.5l.9.5 1.8-1.8-.5-.9c.2-.4.4-.8.5-1.2l.9-.4z" />,
  search: <path d="m16.5 16.5 3 3M11 18a7 7 0 1 1 0-14 7 7 0 0 1 0 14z" />,
  recent: <path d="M12 6v6l4 2M12 4a8 8 0 1 0 8 8" />,
  user: <path d="M12 12a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zm-6 8a6 6 0 0 1 12 0" />,
  cluster: <path d="M12 4l7 4-7 4-7-4 7-4zm7 8-7 4-7-4m14 4-7 4-7-4" />,
};

export function Icon({ name, className, ...props }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.65"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
      {...props}
    >
      {iconPaths[name]}
    </svg>
  );
}

export function KafkaDeskMark({ className, ...props }: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.15"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
      {...props}
    >
      <path d="M12 3.1 19.4 7.4v9.2L12 20.9 4.6 16.6V7.4z" fill="currentColor" opacity="0.2" stroke="currentColor" />
      <path d="M7.4 9h9.2" />
      <path d="M12 9v6.6" />
      <path d="M9 15.2 12 18.4l3-3.2" />
      <circle cx="7.4" cy="9" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="16.6" cy="9" r="1.1" fill="currentColor" stroke="none" />
      <circle cx="12" cy="18.4" r="1.1" fill="currentColor" stroke="none" />
    </svg>
  );
}
