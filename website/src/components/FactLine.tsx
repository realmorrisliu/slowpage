import { For } from 'solid-js'

/**
 * A ` · `-separated fact line. Each fact stays on one line, so a narrow
 * viewport wraps between facts, never inside “免费开源” or “10 lines”.
 */
export function FactLine(props: { items: readonly (string | null | undefined)[] }) {
  const items = () => props.items.filter((item): item is string => Boolean(item))
  return (
    <For each={items()}>
      {(item, i) => (
        <>
          {i() > 0 ? ' · ' : ''}
          <span class="whitespace-nowrap">{item}</span>
        </>
      )}
    </For>
  )
}
