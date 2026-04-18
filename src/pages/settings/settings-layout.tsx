import { NavLink, Outlet } from 'react-router-dom';
import { settingsNavigation } from '@/app/navigation';
import { PageFrame } from '@/components/layout/page-frame';

export function SettingsLayout() {
  return (
    <PageFrame 
      eyebrow="本地配置中心"
      title="设置"
      description="当前以集群配置为主，提供全面的设置管理。"
      summary={
        <nav className="subnav-bar xl:col-span-4" aria-label="设置子导航">
          <div className="subnav-shell">
            {settingsNavigation.map((item) => (
              <NavLink key={item.path} to={item.path} className="subnav-link">
                {item.label}
              </NavLink>
            ))}
          </div>
        </nav>
      }
    >
      <Outlet />
    </PageFrame>
  );
}
