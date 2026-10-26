<?php

namespace __FerruleNs__\Core\Exceptions;

class __FerruleName__Exception extends \Exception
{
    /** @var string */
    protected const DESC = '__FerruleName__ Error';

    public function __construct(string $message, int $code = 0, ?\Throwable $previous = null)
    {
        parent::__construct($this::DESC.PHP_EOL.$message, $code, $previous);
    }
}
