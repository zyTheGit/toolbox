/** 版本检查模型。 */

/** 只回答“有没有新版本”，下载安装由用户在浏览器里完成。 */
export interface UpdateCheck {
  currentVersion: string;
  latestVersion: string;
  available: boolean;
  /** 当前版本高于线上最新版：正常发版不会出现，出现即说明版本号被误标。 */
  currentNewer: boolean;
  /** Release 页面（不是安装包直链）。 */
  pageUrl: string;
  publishedAt?: string | null;
}
