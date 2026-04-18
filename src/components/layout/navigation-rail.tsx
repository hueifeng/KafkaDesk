import { NavLink } from 'react-router-dom';
import { primaryNavigation, supportNavigation } from '@/app/navigation';
import { Icon, KafkaDeskMark } from '@/components/ui/icons';

export function NavigationRail() {
  return (
    <aside className="navigation-rail">
      <div className="brand-mark">
        <div className="brand-glyph">
          <KafkaDeskMark className="h-[21px] w-[21px]" />
        </div>
        <div className="brand-copy">
          <span className="brand-eyebrow">Kafka 事件流排障工具</span>
          <span className="brand-title">KafkaDesk</span>
        </div>
      </div>

      <p className="nav-group-label">功能</p>
      <nav aria-label="主导航">
        {primaryNavigation.map((item) => (
          <NavLink key={item.path} to={item.path} className="nav-link">
            <span className="nav-icon" aria-hidden="true">
              <Icon name={item.icon} className="h-4 w-4" />
            </span>
            <span className="nav-copy">
              <span className="nav-title">{item.label}</span>
            </span>
          </NavLink>
        ))}
      </nav>

      <p className="nav-group-label">设置</p>
      <nav aria-label="辅助导航" className="mt-auto">
        {supportNavigation.map((item) => (
          <NavLink key={item.path} to={item.path} className="nav-link">
            <span className="nav-icon" aria-hidden="true">
              <Icon name={item.icon} className="h-4 w-4" />
            </span>
            <span className="nav-copy">
              <span className="nav-title">{item.label}</span>
            </span>
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
