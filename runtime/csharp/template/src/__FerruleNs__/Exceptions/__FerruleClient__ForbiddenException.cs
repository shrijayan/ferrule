using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__ForbiddenException : __FerruleClient__4xxException
{
    public __FerruleClient__ForbiddenException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
