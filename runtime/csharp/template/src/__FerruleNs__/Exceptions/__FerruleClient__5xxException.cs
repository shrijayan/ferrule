using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__5xxException : __FerruleClient__ApiException
{
    public __FerruleClient__5xxException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
