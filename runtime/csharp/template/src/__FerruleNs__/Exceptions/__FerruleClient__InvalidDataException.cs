using System;

namespace __FerruleNs__.Exceptions;

public class __FerruleClient__InvalidDataException : __FerruleClient__Exception
{
    public __FerruleClient__InvalidDataException(string message, Exception? innerException = null)
        : base(message, innerException) { }
}
