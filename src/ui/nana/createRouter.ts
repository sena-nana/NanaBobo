import {
  createMemoryHistory,
  createRouter,
  type RouteRecordRaw,
} from "vue-router";

/**
 * 路由只承担导航状态:页面常驻在 NanaShell 内,由 route.path 驱动 v-show
 * 可见性。每条记录必须有占位 component,否则匹配不到、会落入重定向死循环。
 */
export function createNanaRouter() {
  const stub = { render: () => null };
  const routes: RouteRecordRaw[] = [
    { path: "/", component: stub },
    { path: "/assistant", component: stub },
    { path: "/stats", component: stub },
    { path: "/history", component: stub },
    { path: "/settings", component: stub },
    { path: "/:pathMatch(.*)*", redirect: "/" },
  ];
  return createRouter({ history: createMemoryHistory(), routes });
}
