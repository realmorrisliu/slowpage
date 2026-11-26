// Real IconMCP outline icons from Slowpage's `assets/icons/` (stroke-width 1.25, radius 2).

import type { JSX } from '@solidjs/web'

type IconProps = { class?: string }

const base = {
  width: '16',
  height: '16',
  viewBox: '0 0 16 16',
  fill: 'none',
  stroke: 'currentColor',
  'stroke-width': '1.25',
  'stroke-linecap': 'round' as const,
  'stroke-linejoin': 'round' as const,
  'aria-hidden': 'true' as const,
}

export function IconList(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <circle cx="2.75" cy="3.75" r="0.825" fill="currentColor" stroke="none" />
      <circle cx="2.75" cy="8" r="0.825" fill="currentColor" stroke="none" />
      <circle cx="2.75" cy="12.25" r="0.825" fill="currentColor" stroke="none" />
      <line x1="6" y1="3.75" x2="13.25" y2="3.75" />
      <line x1="6" y1="8" x2="13.25" y2="8" />
      <line x1="6" y1="12.25" x2="13.25" y2="12.25" />
    </svg>
  )
}

export function IconMaximize(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M2.25 6 L2.25 3.25 C2.25 2.698 2.698 2.25 3.25 2.25 L6 2.25" />
      <path d="M10 2.25 L12.75 2.25 C13.302 2.25 13.75 2.698 13.75 3.25 L13.75 6" />
      <path d="M13.75 10 L13.75 12.75 C13.75 13.302 13.302 13.75 12.75 13.75 L10 13.75" />
      <path d="M6 13.75 L3.25 13.75 C2.698 13.75 2.25 13.302 2.25 12.75 L2.25 10" />
    </svg>
  )
}

export function IconBook(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M3.25 12.5 V3.5 A1.5 1.5 0 0 1 4.75 2 H12.75 V14 H4.75 A1.5 1.5 0 0 1 3.25 12.5 Z" />
      <path d="M3.25 12.5 A1.5 1.5 0 0 1 4.75 11 H12.75" />
    </svg>
  )
}

export function IconBookOpen(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M8 4 C6.5 3 4.5 2.75 2 2.75 V12.25 C4.5 12.25 6.5 12.5 8 13.5 C9.5 12.5 11.5 12.25 14 12.25 V2.75 C11.5 2.75 9.5 3 8 4 Z" />
      <line x1="8" y1="4" x2="8" y2="13.5" />
    </svg>
  )
}

export function IconSidebarLeft(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <rect x="2.25" y="2.75" width="11.5" height="10.5" rx="2" />
      <line x1="6" y1="2.75" x2="6" y2="13.25" />
    </svg>
  )
}

export function IconSidebarRight(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <rect x="2.25" y="2.75" width="11.5" height="10.5" rx="2" />
      <line x1="10" y1="2.75" x2="10" y2="13.25" />
    </svg>
  )
}

export function IconLibrary(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <line x1="2.75" y1="2.25" x2="2.75" y2="13.75" />
      <line x1="5.75" y1="2.25" x2="5.75" y2="13.75" />
      <path d="M8.769 3.276 L10.233 2.849 C10.657 2.725 11.1 2.968 11.225 3.392 L13.775 12.106 C13.899 12.531 13.656 12.975 13.231 13.099 L11.767 13.526 C11.343 13.65 10.9 13.407 10.775 12.983 L8.225 4.269 C8.101 3.844 8.344 3.4 8.769 3.276 Z" />
    </svg>
  )
}

export function IconNote(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M4.5 2.5 L11.5 2.5 C12.605 2.5 13.5 3.395 13.5 4.5 L13.5 9.5 L9.5 13.5 L4.5 13.5 C3.395 13.5 2.5 12.605 2.5 11.5 L2.5 4.5 C2.5 3.395 3.395 2.5 4.5 2.5 Z" />
      <path d="M13.5 9.5 L10.3 9.5 C9.858 9.5 9.5 9.858 9.5 10.3 L9.5 13.5" />
    </svg>
  )
}

export function IconFile(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M5.25 2 L9 2 L12.75 5.75 L12.75 12 C12.75 13.105 11.855 14 10.75 14 L5.25 14 C4.145 14 3.25 13.105 3.25 12 L3.25 4 C3.25 2.895 4.145 2 5.25 2 Z" />
      <path d="M9 2 L9 4.75 C9 5.302 9.448 5.75 10 5.75 L12.75 5.75" />
    </svg>
  )
}

export function IconFolder(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M4.25 3.25 L5.33 3.25 C5.914 3.25 6.469 3.505 6.849 3.948 L7.151 4.302 C7.531 4.745 8.086 5 8.67 5 L11.75 5 C12.855 5 13.75 5.895 13.75 7 L13.75 10.75 C13.75 11.855 12.855 12.75 11.75 12.75 L4.25 12.75 C3.145 12.75 2.25 11.855 2.25 10.75 L2.25 5.25 C2.25 4.145 3.145 3.25 4.25 3.25 Z" />
    </svg>
  )
}

export function IconSearch(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <circle cx="7" cy="7" r="4.75" />
      <line x1="10.5" y1="10.5" x2="13.75" y2="13.75" />
    </svg>
  )
}

export function IconSettings(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M6.663 3.269 L6.631 2.646 C6.611 2.246 6.93 1.91 7.33 1.91 L8.67 1.91 C9.07 1.91 9.389 2.246 9.369 2.646 L9.337 3.269 C9.324 3.532 9.459 3.779 9.686 3.911 L10.698 4.495 C10.926 4.626 11.208 4.619 11.428 4.476 L11.953 4.137 C12.289 3.92 12.739 4.028 12.939 4.375 L13.609 5.535 C13.809 5.882 13.678 6.326 13.322 6.508 L12.766 6.793 C12.532 6.912 12.385 7.153 12.385 7.416 L12.385 8.584 C12.385 8.847 12.532 9.088 12.766 9.207 L13.322 9.492 C13.678 9.674 13.809 10.118 13.609 10.465 L12.939 11.625 C12.739 11.972 12.289 12.08 11.953 11.863 L11.428 11.524 C11.208 11.381 10.926 11.374 10.698 11.505 L9.686 12.089 C9.459 12.221 9.324 12.468 9.337 12.731 L9.369 13.354 C9.389 13.754 9.07 14.09 8.67 14.09 L7.33 14.09 C6.93 14.09 6.611 13.754 6.631 13.354 L6.663 12.731 C6.676 12.468 6.541 12.221 6.314 12.089 L5.302 11.505 C5.074 11.374 4.792 11.381 4.572 11.524 L4.047 11.863 C3.711 12.08 3.261 11.972 3.061 11.625 L2.391 10.465 C2.191 10.118 2.322 9.674 2.678 9.492 L3.234 9.207 C3.468 9.088 3.615 8.847 3.615 8.584 L3.615 7.416 C3.615 7.153 3.468 6.912 3.234 6.793 L2.678 6.508 C2.322 6.326 2.191 5.882 2.391 5.535 L3.061 4.375 C3.261 4.028 3.711 3.92 4.047 4.137 L4.572 4.476 C4.792 4.619 5.074 4.626 5.302 4.495 L6.314 3.911 C6.541 3.779 6.676 3.532 6.663 3.269 Z" />
      <circle cx="8" cy="8" r="2" />
    </svg>
  )
}

export function IconChevronDown(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <g transform="rotate(90 8 8)">
        <path d="M5.875 3.75 L10.125 8 L5.875 12.25" />
      </g>
    </svg>
  )
}

export function IconArrowUp(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <g transform="rotate(-90 8 8)">
        <line x1="2.75" y1="8" x2="13.25" y2="8" />
        <path d="M8.5 3.25 L13.25 8 L8.5 12.75" />
      </g>
    </svg>
  )
}

export function IconCheck(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M2.75 8.5 L6.25 12 L13.25 4.5" />
    </svg>
  )
}

export function IconEdit(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M10.957 3.457 L12.543 5.043 C12.933 5.433 12.933 6.067 12.543 6.457 L5.793 13.207 C5.605 13.395 5.351 13.5 5.086 13.5 L2.5 13.5 L2.5 10.914 C2.5 10.649 2.605 10.395 2.793 10.207 L9.543 3.457 C9.933 3.067 10.567 3.067 10.957 3.457 Z" />
      <line x1="8.75" y1="4.25" x2="11.75" y2="7.25" />
    </svg>
  )
}

export function IconBookmark(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <path d="M5.75 2.25 L10.25 2.25 C11.355 2.25 12.25 3.145 12.25 4.25 L12.25 13.75 L8 10.5 L3.75 13.75 L3.75 4.25 C3.75 3.145 4.645 2.25 5.75 2.25 Z" />
    </svg>
  )
}

export function IconSliders(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <line x1="2.25" y1="4.75" x2="7.75" y2="4.75" />
      <circle cx="9.75" cy="4.75" r="2" />
      <line x1="11.75" y1="4.75" x2="13.75" y2="4.75" />
      <line x1="2.25" y1="11.25" x2="4.25" y2="11.25" />
      <circle cx="6.25" cy="11.25" r="2" />
      <line x1="8.25" y1="11.25" x2="13.75" y2="11.25" />
    </svg>
  )
}

export function IconSortDesc(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <line x1="2.25" y1="4" x2="9.25" y2="4" />
      <line x1="2.25" y1="8" x2="7.25" y2="8" />
      <line x1="2.25" y1="12" x2="5.25" y2="12" />
      <line x1="12" y1="3.25" x2="12" y2="12.75" />
      <path d="M10 10.75 L12 12.75 L14 10.75" />
    </svg>
  )
}

export function IconPlus(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <line x1="8" y1="2.75" x2="8" y2="13.25" />
      <line x1="2.75" y1="8" x2="13.25" y2="8" />
    </svg>
  )
}

export function IconGrid(props: IconProps): JSX.Element {
  return (
    <svg {...base} class={props.class}>
      <rect x="2.25" y="2.25" width="4.75" height="4.75" rx="1.25" />
      <rect x="9" y="2.25" width="4.75" height="4.75" rx="1.25" />
      <rect x="2.25" y="9" width="4.75" height="4.75" rx="1.25" />
      <rect x="9" y="9" width="4.75" height="4.75" rx="1.25" />
    </svg>
  )
}

export function SlowpageAvatar(props: { class?: string }): JSX.Element {
  return (
    <img
      src="/slowpage-icon.png"
      alt=""
      aria-hidden="true"
      class={`shrink-0 drop-shadow-[0_1px_2px_rgba(0,0,0,0.12)] ${props.class ?? 'size-7'}`}
    />
  )
}
