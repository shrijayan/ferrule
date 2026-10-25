<?php

namespace __FerruleNs__\Core\Exceptions;

class RateLimitException extends APIStatusException
{
    /** @var string */
    protected const DESC = '__FerruleName__ Rate Limit Exception';
}
