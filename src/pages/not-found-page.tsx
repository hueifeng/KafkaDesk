import { Link } from 'react-router-dom';
import { PageFrame } from '@/components/layout/page-frame';
import { EmptyState } from '@/components/ui/empty-state';

export function NotFoundPage() {
  return (
    <PageFrame
      eyebrow="路由兜底"
      title="页面不存在"
        description="当前 KafkaDesk 壳层中没有这个页面。"
      tags={[{ label: '404', tone: 'danger' }]}
    >
      <EmptyState
        title="返回工作区"
        description="请使用左侧稳定导航回到已经实现的页面。"
        action={
          <Link to="/overview" className="button-shell" data-variant="primary">
            返回概览
          </Link>
        }
      />
    </PageFrame>
  );
}
