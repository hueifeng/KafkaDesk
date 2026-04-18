import type { IconName } from '@/components/ui/icons';

export type NavigationItem = {
  label: string;
  description: string;
  path: string;
  icon: IconName;
};

export const primaryNavigation: NavigationItem[] = [
  {
    label: '概览',
    description: '集群总览与排障入口',
    path: '/overview',
    icon: 'overview',
  },
  {
    label: '主题',
    description: '主题列表与筛选',
    path: '/topics',
    icon: 'topics',
  },
  {
    label: '消费组',
    description: '积压诊断入口',
    path: '/groups',
    icon: 'groups',
  },
  {
    label: '消息',
    description: '受控消息查询',
    path: '/messages',
    icon: 'messages',
  },
  {
    label: '回放',
    description: '受控回放流程',
    path: '/replay',
    icon: 'replay',
  },
  {
    label: '追踪',
    description: '事件链路定位',
    path: '/trace',
    icon: 'trace',
  },
  {
    label: '保存的查询',
    description: '复用排查方案',
    path: '/saved-queries',
    icon: 'saved',
  },
  {
    label: '审计',
    description: '敏感操作记录',
    path: '/audit',
    icon: 'audit',
  },
];

export const supportNavigation: NavigationItem[] = [
  {
    label: '设置',
    description: '本地配置与策略',
    path: '/settings',
    icon: 'settings',
  },
];

export const settingsNavigation: NavigationItem[] = [
  {
    label: '集群配置',
    description: '连接、认证与 TLS',
    path: '/settings/cluster-profiles',
    icon: 'cluster',
  },
  {
    label: '模式注册表',
    description: '解码服务连接',
    path: '/settings/schema-registry',
    icon: 'topics',
  },
  {
    label: '应用偏好',
    description: '默认项与显示密度',
    path: '/settings/preferences',
    icon: 'overview',
  },
  {
    label: '关联规则',
    description: '追踪键匹配策略',
    path: '/settings/correlation-rules',
    icon: 'trace',
  },
  {
    label: '回放策略',
    description: '写入类操作约束',
    path: '/settings/replay-policy',
    icon: 'replay',
  },
];
