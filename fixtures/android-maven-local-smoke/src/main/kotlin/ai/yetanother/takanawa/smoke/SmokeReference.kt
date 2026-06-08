package ai.yetanother.takanawa.smoke

import ai.yetanother.takanawa.DownloadConfig
import ai.yetanother.takanawa.DownloadPhase
import ai.yetanother.takanawa.TakanawaStatus

class SmokeReference {
    fun config(url: String, targetPath: String): DownloadConfig =
        DownloadConfig(url = url, targetPath = targetPath)

    fun createdPhase(): DownloadPhase = DownloadPhase.CREATED

    fun okStatus(): TakanawaStatus = TakanawaStatus.OK
}
