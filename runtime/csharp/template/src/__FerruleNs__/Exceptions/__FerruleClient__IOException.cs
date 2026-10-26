using System;
using System.Net.Http;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__IOException : __FerruleClient__Exception
{
    public new HttpRequestException InnerException
    {
        get
        {
            if (base.InnerException == null)
            {
                throw new ArgumentNullException();
            }
            return (HttpRequestException)base.InnerException;
        }
    }

    public __FerruleClient__IOException(string message, HttpRequestException? innerException = null)
        : base(message, innerException) { }
}
