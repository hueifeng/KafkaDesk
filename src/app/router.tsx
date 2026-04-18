import { Navigate, createBrowserRouter } from 'react-router-dom';
import { AppShell } from '@/components/layout/app-shell';
import { AuditPage } from '@/pages/audit-page';
import { GroupsPage } from '@/pages/groups-page';
import { GroupDetailPage } from '@/pages/group-detail-page';
import { MessageDetailPage } from '@/pages/message-detail-page';
import { MessagesPage } from '@/pages/messages-page';
import { NotFoundPage } from '@/pages/not-found-page';
import { OverviewPage } from '@/pages/overview-page';
import { ReplayPage } from '@/pages/replay-page';
import { SavedQueriesPage } from '@/pages/saved-queries-page';
import { ClusterProfilesPage } from '@/pages/settings/cluster-profiles-page';
import { CorrelationRulesPage } from '@/pages/settings/correlation-rules-page';
import { PreferencesPage } from '@/pages/settings/preferences-page';
import { ReplayPolicyPage } from '@/pages/settings/replay-policy-page';
import { SchemaRegistryPage } from '@/pages/settings/schema-registry-page';
import { SettingsLayout } from '@/pages/settings/settings-layout';
import { TopicDetailPage } from '@/pages/topic-detail-page';
import { TopicsPage } from '@/pages/topics-page';
import { TracePage } from '@/pages/trace-page';

export const router = createBrowserRouter([
  {
    path: '/',
    element: <AppShell />,
    children: [
      {
        index: true,
        element: <Navigate to="/overview" replace />,
      },
      {
        path: 'overview',
        element: <OverviewPage />,
      },
      {
        path: 'topics',
        element: <TopicsPage />,
      },
      {
        path: 'topics/:topicName',
        element: <TopicDetailPage />,
      },
      {
        path: 'groups',
        element: <GroupsPage />,
      },
      {
        path: 'groups/:groupName',
        element: <GroupDetailPage />,
      },
      {
        path: 'messages',
        element: <MessagesPage />,
      },
      {
        path: 'messages/:topic/:partition/:offset',
        element: <MessageDetailPage />,
      },
      {
        path: 'replay',
        element: <ReplayPage />,
      },
      {
        path: 'trace',
        element: <TracePage />,
      },
      {
        path: 'saved-queries',
        element: <SavedQueriesPage />,
      },
      {
        path: 'audit',
        element: <AuditPage />,
      },
      {
        path: 'settings',
        element: <SettingsLayout />,
        children: [
          {
            index: true,
            element: <Navigate to="cluster-profiles" replace />,
          },
          {
            path: 'cluster-profiles',
            element: <ClusterProfilesPage />,
          },
          {
            path: 'schema-registry',
            element: <SchemaRegistryPage />,
          },
          {
            path: 'preferences',
            element: <PreferencesPage />,
          },
          {
            path: 'correlation-rules',
            element: <CorrelationRulesPage />,
          },
          {
            path: 'replay-policy',
            element: <ReplayPolicyPage />,
          },
        ],
      },
      {
        path: '*',
        element: <NotFoundPage />,
      },
    ],
  },
]);
