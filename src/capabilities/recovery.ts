import { inject, type InjectionKey } from "vue";
import type { RecoveryErrorModel } from "../ui/contract";
import type { TemplateAppCapability } from "../ui/types";

export type AppErrorMapper = (error: unknown, retry: () => void | Promise<void>) => RecoveryErrorModel;
const recoveryKey: InjectionKey<AppErrorMapper> = Symbol("templateRecoveryMapper");

export function mapAppError(error: unknown, retry: () => void | Promise<void>): RecoveryErrorModel {
  const details = error instanceof Error ? error.message : String(error);
  return {
    title: "操作未完成",
    impact: "当前修改尚未保存，请重试或返回后继续编辑。",
    actions: [{ id: "retry", label: "重试", run: retry }],
    technicalDetails: details,
  };
}

export function recoveryCapability(mapper: AppErrorMapper = mapAppError): TemplateAppCapability {
  return { id: "recovery", install: (app) => app.provide(recoveryKey, mapper) };
}

export function useErrorMapper(): AppErrorMapper {
  const mapper = inject(recoveryKey);
  if (!mapper) throw new Error("Recovery capability is not installed.");
  return mapper;
}
