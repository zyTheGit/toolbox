/**
 * 数据库页面请求序号闸门。
 *
 * 同一通道内每次发起操作都会递增序号；响应回来时若序号已不是最新，
 * 说明期间又发起了新的操作（切表 / 切库 / 断连），旧结果直接丢弃，
 * 避免慢请求覆盖新结果。
 */

export type DbSeqChannel = "open" | "selectDatabase" | "query" | "close";

export interface DbSeq {
  /** 开启新一轮请求并返回本次序号 */
  begin(channel: DbSeqChannel): number;
  /** 响应是否仍是该通道最新一次请求 */
  isLatest(channel: DbSeqChannel, token: number): boolean;
  /** 作废某通道上尚未返回的请求 */
  invalidate(channel: DbSeqChannel): void;
  /** 作废所有通道上尚未返回的请求 */
  invalidateAll(): void;
}

const CHANNELS: DbSeqChannel[] = ["open", "selectDatabase", "query", "close"];

export function createDbSeq(): DbSeq {
  const counters: Record<DbSeqChannel, number> = {
    open: 0,
    selectDatabase: 0,
    query: 0,
    close: 0,
  };
  return {
    begin(channel) {
      counters[channel] += 1;
      return counters[channel];
    },
    isLatest(channel, token) {
      return counters[channel] === token;
    },
    invalidate(channel) {
      counters[channel] += 1;
    },
    invalidateAll() {
      for (const channel of CHANNELS) {
        counters[channel] += 1;
      }
    },
  };
}
