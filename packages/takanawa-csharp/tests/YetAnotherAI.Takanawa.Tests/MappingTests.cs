using Xunit;

namespace YetAnotherAI.Takanawa.Tests
{
    public sealed class MappingTests
    {
        [Fact]
        public void UnknownStatusMapsToInternal()
        {
            Assert.Equal(TakanawaStatus.Internal, NativeConversions.NormalizeStatus((TakanawaStatus)12345));
        }

        [Fact]
        public void UnknownPhaseMapsToFailed()
        {
            Assert.Equal(DownloadPhase.Failed, NativeConversions.ToDownloadPhase(12345));
        }

        [Fact]
        public void NewPhaseValuesMapToNamedCases()
        {
            Assert.Equal(DownloadPhase.Verifying, NativeConversions.ToDownloadPhase(10));
        }

        [Fact]
        public void ClosedHandleThrowsAlignedException()
        {
            using var download = new TakanawaDownload(new TakanawaDownloadHandle(System.IntPtr.Zero));

            var exception = Assert.Throws<TakanawaException>(() => download.Snapshot());
            Assert.Equal(TakanawaStatus.InvalidConfig, exception.Status);
            Assert.Equal("download is closed", exception.Message);
        }

        [Fact]
        public void DoubleDisposeIsAllowedForClosedHandle()
        {
            using var download = new TakanawaDownload(new TakanawaDownloadHandle(System.IntPtr.Zero));

            download.Dispose();
            download.Dispose();
        }
    }
}
