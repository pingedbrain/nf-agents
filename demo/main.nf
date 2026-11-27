// SPDX-License-Identifier: Apache-2.0
// Local demo pipeline: generate -> transform -> summarize, plus one AI step (QC_NOTE) that checks
// the transformed file. Synthetic data only; research use, no clinical claims.
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
    path 'summary.txt'

    script:
    """
    echo "lines: \$(wc -l < ${upper})" > summary.txt
    """
}

record QcVerdict {
    answer: String
    confidence: Float
}

agent QC_NOTE {
    model 'openai/gpt-5-mini'

    input:
    text: String

    output:
    verdict: QcVerdict

    prompt:
    "Does this synthetic file contain any lowercase letters? Answer yes or no.\n${text}"
}

workflow {
    main:
    upper = TRANSFORM(GENERATE(file(params.input), params.greeting))
    summary = SUMMARIZE(upper)
    QC_NOTE(upper.map { it.text }).view { "QC verdict: ${it}" }

    publish:
    upper = upper
    summary = summary
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
