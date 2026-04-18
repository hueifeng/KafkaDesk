type DecodeStatusTone = 'success' | 'warning' | 'danger' | 'info' | 'muted';

type DecodeStatusDefinition = {
  status: string;
  label: string;
  tone: DecodeStatusTone;
  description: string;
};

export const DECODE_STATUS_DEFINITIONS: DecodeStatusDefinition[] = [
  {
    status: 'avro-decoded',
    label: 'Avro 已解码',
    tone: 'success',
    description: '消息已通过 Schema Registry 获取到可用 Avro schema，并成功解码为可展示内容。',
  },
  {
    status: 'schema-unsupported',
    label: 'Schema 类型未支持',
    tone: 'warning',
    description: '已识别到受 Registry 管理的消息，但当前 schemaType 还未纳入运行时解码支持。',
  },
  {
    status: 'schema-auth-unsupported',
    label: 'Registry 认证未支持',
    tone: 'warning',
    description: '消息需要带认证访问 Schema Registry，但当前运行路径尚未支持该认证模式。',
  },
  {
    status: 'schema-auth-failed',
    label: 'Registry 凭据不可用',
    tone: 'danger',
    description: 'Schema Registry 需要认证，但 keyring 中缺少可用 secret，或 secret 格式不正确。',
  },
  {
    status: 'schema-references-unsupported',
    label: 'Schema 引用未支持',
    tone: 'warning',
    description: '当前版本尚未解析带 references 的 Avro schema。',
  },
  {
    status: 'schema-registry-error',
    label: 'Registry 读取失败',
    tone: 'danger',
    description: '已检测到 Schema Registry 关联，但读取 schema 元数据失败。',
  },
  {
    status: 'schema-decode-failed',
    label: 'Schema 解码失败',
    tone: 'danger',
    description: 'Schema 已返回，但当前消息体与 schema 不匹配，或运行时解码过程失败。',
  },
  {
    status: 'utf8',
    label: 'UTF-8 文本',
    tone: 'info',
    description: '消息体可直接按 UTF-8 文本查看，但当前没有结构化 schema 解码结果。',
  },
  {
    status: 'binary',
    label: '二进制',
    tone: 'muted',
    description: '消息体不是可直接展示的 UTF-8 文本，也没有可用 schema 解码结果。',
  },
  {
    status: 'empty',
    label: '空消息',
    tone: 'muted',
    description: '消息体为空，因此没有可展示的原始内容或解码内容。',
  },
];

const decodeStatusMap = new Map(DECODE_STATUS_DEFINITIONS.map((definition) => [definition.status, definition]));

export function getDecodeStatusDefinition(status: string) {
  return decodeStatusMap.get(status);
}

export function formatDecodeStatus(status: string) {
  return getDecodeStatusDefinition(status)?.label ?? status;
}

export function describeDecodeStatus(status: string) {
  return getDecodeStatusDefinition(status)?.description ?? '当前没有可展示的解码结果';
}
