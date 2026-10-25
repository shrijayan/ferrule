<?php

namespace __FerruleNs__\Core\Exceptions;

class PermissionDeniedException extends APIStatusException
{
    /** @var string */
    protected const DESC = '__FerruleName__ Permission Denied Exception';
}
