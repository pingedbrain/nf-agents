// SPDX-License-Identifier: Apache-2.0
// Minimal 3-process pipeline used to generate Nextflow lineage fixtures.
// No real data: generate -> transform -> summarize.
params.input    = "${projectDir}/data/seed.txt"
params.greeting = 'hello'

process GENERATE {
    input:
    path seed
    val greeting

    output:
    path 'raw.txt'

    script:
    """
    while read -r w; do echo "${greeting} \$w"; done < ${seed} > raw.txt
    """
}

process TRANSFORM {
    input:
    path raw

    output:
    path 'upper.txt'

    script:
    """
    tr a-z A-Z < ${raw} > upper.txt
    """
}

process SUMMARIZE {
    input:
    path upper

    output:
    path 'summary.txt', emit: summary
    val 'done', emit: status

    script:
    """
    echo "lines: \$(wc -l < ${upper})" > summary.txt
    """
}

workflow {
    main:
    raw = GENERATE(file(params.input), params.greeting)
    upper = TRANSFORM(raw)
    SUMMARIZE(upper)

    publish:
    upper = upper
    summary = SUMMARIZE.out.summary
}

output {
    upper {
        path 'upper'
    }
    summary {
        path '.'
        label 'summary'
    }
}
