using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__UnauthorizedException : __FerruleClient__4xxException
{
    public __FerruleClient__UnauthorizedException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
