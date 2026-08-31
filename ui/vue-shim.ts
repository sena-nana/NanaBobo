/**
 * vue 别名 shim:NanaUI 渲染器要求 bundle 只含 @vue/runtime-core,
 * 但 SFC 模板修饰符(@click.stop / @submit.prevent)编译产物依赖
 * runtime-dom 的 withModifiers。这里按需补齐 Nana 事件对象支持的最小子集。
 */
export * from "@vue/runtime-core";

type GuardedEvent = {
  stopPropagation?: () => void;
  preventDefault?: () => void;
  target?: unknown;
  currentTarget?: unknown;
} | undefined;

export function withModifiers(
  handler: (event?: any, ...args: unknown[]) => unknown,
  modifiers: string[],
) {
  return (event: GuardedEvent, ...args: unknown[]) => {
    for (const modifier of modifiers) {
      switch (modifier) {
        case "stop":
          event?.stopPropagation?.();
          break;
        case "prevent":
          event?.preventDefault?.();
          break;
        case "self":
          if (event && event.target !== event.currentTarget) return;
          break;
      }
    }
    return handler(event, ...args);
  };
}

type StylefulElement = { style: { display: string } };

/** v-show 的最小实现:直接切换行内 display(行内样式补丁路径可靠)。 */
export const vShow = {
  beforeMount(el: StylefulElement, { value }: { value: unknown }) {
    el.style.display = value ? "" : "none";
  },
  updated(el: StylefulElement, { value }: { value: unknown }) {
    el.style.display = value ? "" : "none";
  },
};
