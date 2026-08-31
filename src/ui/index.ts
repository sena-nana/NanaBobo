/**
 * NanaBobo UI 门面(NanaUI 宿主版)。
 *
 * 对外导出签名与迁移前保持一致,业务页面不直接依赖 @nanaui/*。
 * 叶子控件用 DOM 渲染 + 门面样式(nana-styles.css)保持 Lilia 视觉;
 * 壳层(标题栏/窗口控制/原生外观)由 NanaUI 原生能力提供。
 */
import { defineComponent, h, ref, watch, type PropType } from "vue";

const Button = defineComponent({
  name: "UiButton",
  props: {
    variant: { type: String as PropType<"primary" | "ghost" | "text" | "danger">, default: "ghost" },
    size: { type: String as PropType<"sm" | "md" | "lg">, default: "md" },
    type: { type: String as PropType<"button" | "submit" | "reset">, default: "button" },
    disabled: { type: Boolean, default: false },
    loading: { type: Boolean, default: false },
    invalid: { type: Boolean, default: false },
    agentId: { type: String, default: undefined },
    iconOnly: { type: Boolean, default: false },
  },
  emits: ["click"],
  setup(props, { slots, emit }) {
    return () =>
      h(
        "button",
        {
          type: props.type,
          class: [
            "ui-button",
            `ui-button--${props.variant}`,
            `ui-button--${props.size}`,
            { "ui-button--icon-only": props.iconOnly, "is-busy": props.loading },
          ],
          disabled: props.disabled || props.loading,
          "aria-busy": props.loading || undefined,
          "aria-invalid": props.invalid || undefined,
          "data-agent-id": props.agentId,
          onClick: (event: MouseEvent) => {
            if (props.disabled || props.loading) return;
            emit("click", event);
          },
        },
        [slots.icon?.(), slots.default ? h("span", { class: "ui-button__label" }, slots.default()) : null],
      );
  },
});

const Card = defineComponent({
  name: "UiCard",
  props: {
    title: { type: String, default: undefined },
    variant: { type: String as PropType<"surface" | "default" | "outlined" | "raised" | "flat" | "interactive">, default: "surface" },
    loading: { type: Boolean, default: false },
    empty: { type: Boolean, default: false },
    agentId: { type: String, default: undefined },
  },
  setup(props, { slots }) {
    return () =>
      h(
        "section",
        {
          class: ["card", "ui-card", `card--${props.variant === "default" ? "surface" : props.variant}`, { empty: props.empty }],
          "data-agent-id": props.agentId,
        },
        [
          props.title || slots.title
            ? h("h2", [h("span", { class: "card-h2__title" }, slots.title ? slots.title() : props.title)])
            : null,
          props.loading ? h("span", { class: "card-title-loader", "aria-busy": "true" }) : null,
          slots.default?.(),
        ],
      );
  },
});

const Input = defineComponent({
  name: "UiInput",
  props: {
    modelValue: { type: String, default: "" },
    type: { type: String, default: "text" },
    size: { type: String as PropType<"sm" | "md" | "lg">, default: "md" },
    name: { type: String, default: undefined },
    placeholder: { type: String, default: undefined },
    readonly: { type: Boolean, default: false },
    required: { type: Boolean, default: false },
    disabled: { type: Boolean, default: false },
    loading: { type: Boolean, default: false },
    invalid: { type: Boolean, default: false },
    ariaLabel: { type: String, default: undefined },
    ariaDescribedby: { type: String, default: undefined },
    agentId: { type: String, default: undefined },
  },
  emits: ["update:modelValue"],
  setup(props, { emit, attrs }) {
    return () =>
      h("input", {
        ...attrs,
        class: ["ui-input", `ui-input--${props.size}`],
        type: props.type,
        value: props.modelValue,
        name: props.name,
        placeholder: props.placeholder,
        readonly: props.readonly,
        required: props.required,
        disabled: props.disabled || props.loading,
        "aria-busy": props.loading || undefined,
        "aria-invalid": props.invalid || undefined,
        "aria-label": props.ariaLabel,
        "aria-describedby": props.ariaDescribedby,
        "data-agent-id": props.agentId,
        onInput: (event: Event) => {
          emit("update:modelValue", (event.target as HTMLInputElement).value);
        },
      });
  },
});

const Stub = (tag: string, name: string) =>
  defineComponent({
    name,
    setup(_, { slots, attrs }) {
      return () => h(tag, attrs, slots.default?.());
    },
  });

const Checkbox = Stub("ui-checkbox", "UiCheckbox");
const EmptyState = Stub("ui-empty-state", "UiEmptyState");
const FormField = Stub("ui-form-field", "UiFormField");
const ListItem = Stub("ui-list-item", "UiListItem");
const Progress = Stub("ui-progress", "UiProgress");
const Select = Stub("ui-select", "UiSelect");
const Skeleton = Stub("ui-skeleton", "UiSkeleton");
const StatusBadge = Stub("ui-status-badge", "UiStatusBadge");
const Switch = Stub("ui-switch", "UiSwitch");
const Tabs = Stub("ui-tabs", "UiTabs");
const Textarea = Stub("ui-textarea", "UiTextarea");
const Toast = Stub("ui-toast", "UiToast");
const ValidationMessage = Stub("ui-validation-message", "UiValidationMessage");
const InteractiveCard = defineComponent({
  name: "UiInteractiveCard",
  props: {
    agentId: { type: String, default: undefined },
  },
  emits: ["click"],
  setup(props, { slots, emit }) {
    return () =>
      h(
        "section",
        {
          class: ["card", "ui-card", "card--interactive", "lilia-interactive-item"],
          "data-agent-id": props.agentId,
          role: "button",
          tabindex: 0,
          onClick: (event: MouseEvent) => emit("click", event),
        },
        slots.default?.(),
      );
  },
});
const IconButton = defineComponent({
  name: "UiIconButton",
  props: {
    variant: { type: String, default: "ghost" },
    size: { type: String as PropType<"sm" | "md" | "lg">, default: "md" },
    label: { type: String, required: true },
    disabled: { type: Boolean, default: false },
    agentId: { type: String, default: undefined },
  },
  emits: ["click"],
  setup(props, { slots, emit }) {
    return () =>
      h(
        "button",
        {
          type: "button",
          class: ["ui-button", `ui-button--${props.variant}`, `ui-button--${props.size}`, "ui-button--icon-only"],
          "aria-label": props.label,
          title: props.label,
          disabled: props.disabled,
          "data-agent-id": props.agentId,
          onClick: (event: MouseEvent) => emit("click", event),
        },
        slots.default?.(),
      );
  },
});

/** 与 @lilia/ui/composables 同名 composable 的 NanaUI 宿主实现。 */
function usePersistentString(key: string, initialValue = "") {
  const value = ref(localStorage.getItem(key) ?? initialValue);
  watch(value, (next) => {
    if (next === "" || next === null) {
      localStorage.removeItem(key);
    } else {
      localStorage.setItem(key, next);
    }
  });
  return value;
}

export {
  Button,
  Card,
  Checkbox,
  EmptyState,
  FormField,
  IconButton,
  Input,
  InteractiveCard,
  ListItem,
  Progress,
  Select,
  Skeleton,
  StatusBadge,
  Switch,
  Tabs,
  Textarea,
  Toast,
  ValidationMessage,
  usePersistentString,
};
