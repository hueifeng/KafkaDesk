import { Outlet, useLocation } from 'react-router-dom';
import React, { useEffect } from 'react';
import { useWorkbenchStore } from '@/app/workbench-store';
import { ContextHeader } from '@/components/layout/context-header';
import { primaryNavigation, supportNavigation, settingsNavigation } from '@/app/navigation';
import { NavigationRail } from '@/components/layout/navigation-rail';

export function AppShell() {
  const location = useLocation();
  const addRecentItem = useWorkbenchStore((state) => state.addRecentItem);
  const recentItems = useWorkbenchStore((state) => state.recentItems);

  const labelMap = React.useMemo(() => Object.fromEntries([
    ...primaryNavigation,
    ...supportNavigation,
    ...settingsNavigation
  ].map((route) => [route.path, route.label])), []);

  useEffect(() => {
    const routeLabel = labelMap[location.pathname];

    if (!routeLabel) {
      return;
    }

    if (recentItems.length > 0 && recentItems[0].path === location.pathname && recentItems[0].label === routeLabel) {
      return;
    }

    addRecentItem({ path: location.pathname, label: routeLabel });
  }, [location.pathname, addRecentItem, recentItems, labelMap]);

  return (
    <div className="app-shell">
      <NavigationRail />
      <div className="shell-body">
        <ContextHeader />
        <main className="page-scroll">
          <div className="workspace-stage">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
