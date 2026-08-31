export interface NanaHostBridge {
  call(name: string, args?: unknown[]): unknown;
  invoke(name: string, args?: unknown[]): Promise<unknown>;
  /** 返回取消监听函数。 */
  on(name: string, listener: (payload: unknown) => void): () => void;
}

/** 事件监听 API 在 Nana.host 上,同步/异步调用在 __nanaHost 上;这里统一收口。 */
export function nanaHost(): NanaHostBridge {
  const viaNana = (globalThis as { Nana?: { host?: NanaHostBridge } }).Nana?.host;
  if (viaNana) {
    return viaNana;
  }
  const bridge = (globalThis as { __nanaHost?: NanaHostBridge }).__nanaHost;
  if (!bridge) {
    throw new Error("缺少 __nanaHost 宿主桥");
  }
  return bridge;
}
