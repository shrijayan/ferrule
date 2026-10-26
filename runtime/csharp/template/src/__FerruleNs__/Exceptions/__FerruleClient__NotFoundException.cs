using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__NotFoundException : __FerruleClient__4xxException
{
    public __FerruleClient__NotFoundException(HttpRequestException? innerException = null)
        : base(innerException) { }
}
