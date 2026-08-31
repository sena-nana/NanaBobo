import { defineComponent, h } from "vue";
import NanaAppearancePanel from "@nanaui/nanavue-components/NanaAppearancePanel";
import appConfig from "../../../app.config.json";

const AboutSection = defineComponent({
  name: "AboutSection",
  setup() {
    return () => [
      h("h2", { class: "about-section__title" }, appConfig.productTitle),
      h("p", { class: "about-section__meta" }, `版本 ${appConfig.version}`),
      h("p", { class: "about-section__desc" }, "B 站直播工具箱：账号登录、房间连接、弹幕助手与数据统计。"),
    ];
  },
});

export const settingsModel = {
  path: "/settings",
  defaultTab: "appearance",
  description: "偏好设置会保存到本地。",
  tabs: [
    { key: "appearance", label: "外观" },
    { key: "about", label: "关于" },
  ],
  sections: {
    appearance: NanaAppearancePanel,
    about: AboutSection,
  },
} as const;
