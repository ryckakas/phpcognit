<?php

/**
 * Cases whose scores the specification determines, so the comparison does not
 * rest on anyone's opinion. The two operator cases are worked examples from
 * SonarSource's paper: a run of like operators is a single increment, and the
 * cost is in switching between them.
 */
class Cases
{
    // if +1, if +2, if +3 = 6
    public function deepChain($a, $b, $c): void
    {
        if ($a) { if ($b) { if ($c) { echo 1; } } }
    }

    // if +1, elseif +1, else +1 = 3; the chain reads linearly, so no nesting penalty
    public function elseChain($a, $b): void
    {
        if ($a) { echo 1; } elseif ($b) { echo 2; } else { echo 3; }
    }

    // if +1, one && run +1 = 2
    public function oneRun($a, $b, $c): void
    {
        if ($a && $b && $c) { echo 1; }
    }

    // if +1, && run +1, || run +1 = 3
    public function twoRuns($a, $b, $c): void
    {
        if ($a && $b || $c) { echo 1; }
    }

    // switch +1 for the whole statement, regardless of arm count
    public function bigSwitch($a): void
    {
        switch ($a) {
            case 1: echo 1; break;
            case 2: echo 2; break;
            case 3: echo 3; break;
        }
    }

    // foreach +1, if +2, then each sibling if at depth 2 costs +3 = 9
    public function siblings($items): void
    {
        foreach ($items as $i) {
            if ($i) {
                if ($i->a) { echo 1; }
                if ($i->b) { echo 2; }
            }
        }
    }
}
