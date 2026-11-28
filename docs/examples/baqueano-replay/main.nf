nextflow.enable.types = true

// Three agents mirroring the captured request shapes: plain text, structured output
// (json_schema, strict) and a process tool (tool-call round trip). Synthetic research content.
agent qa {
    model 'openai/gpt-5-mini'
    instruction 'You are a concise scientific assistant.'

    input:
    question: String

    output:
    stdout()

    prompt:
    """
    Answer briefly: ${question}
    """
}

record Answer {
    answer: String
    confidence: Float
}

agent qa_structured {
    model 'openai/gpt-5-mini'

    input:
    question: String

    output:
    a: Answer

    prompt:
    "Answer briefly: ${question}"
}

process uppercase {
    input:
    text: String

    output:
    result: String

    exec:
    result = text.toUpperCase()
}

agent shouty {
    model 'openai/gpt-5-mini'
    instruction 'To uppercase text call the `uppercase` tool, then reply with only the result.'
    tools 'nf:module_run'

    input:
    request: String

    output:
    stdout()

    prompt:
    "${request}"
}

workflow {
    qa('What is FASTQ format?').view()
    qa_structured('Is FASTQ binary?').view()
    shouty('make this loud: hello').view()
}
