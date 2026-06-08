package ai.yetanother.takanawa

object Takanawa {
    @JvmStatic
    @JvmOverloads
    fun init(maxIo: Int = 0) {
        require(maxIo >= 0) { "maxIo must be greater than or equal to 0" }
        checkStatus(NativeBridge.globalInit(maxIo))
    }

    @JvmStatic
    fun setMaxIo(maxIo: Int) {
        require(maxIo >= 0) { "maxIo must be greater than or equal to 0" }
        checkStatus(NativeBridge.globalSetMaxIo(maxIo))
    }

    @JvmStatic
    fun shutdown() {
        checkStatus(NativeBridge.globalShutdown())
    }
}
