Security Best Practices
=======================

This document provides security guidance for using formatparse in production environments.

Input Validation
----------------

Always validate and sanitize user input before passing it to formatparse:

.. code-block:: python

   import re
   from formatparse import parse
   
   # Validate pattern length
   def safe_parse(pattern, text):
       # Stricter than the library maximum (10,000 characters for patterns); defense in depth.
       if len(pattern) > 1000:  # Set your own limit
           raise ValueError("Pattern too long")
       if len(text) > 1_000_000:  # Set your own limit
           raise ValueError("Input too long")
       
       # Check for suspicious characters
       if '\0' in pattern or '\0' in text:
           raise ValueError("Null bytes not allowed")
       
       return parse(pattern, text)

Pattern Complexity
------------------

Be cautious when parsing patterns from untrusted sources:

.. code-block:: python

   # Safe: Patterns from trusted sources
   pattern = "{name}: {age:d}"
   result = parse(pattern, "Alice: 30")
   
   # Caution: Patterns from user input
   user_pattern = get_user_input()  # Validate before use
   if is_suspicious_pattern(user_pattern):
       raise ValueError("Pattern not allowed")
   
   result = parse(user_pattern, user_text)

The library includes built-in limits to help protect against malicious patterns:
- Maximum pattern length: 10,000 characters
- Maximum input length: 10MB
- Maximum fields: 100
- After each successful regex compilation, the library checks that compilation took at most 200ms of wall-clock time (this does not bound matching time; see below)

Regular Expression Denial of Service (ReDoS)
---------------------------------------------

**Compilation vs matching:** The 200ms check runs only *after* regex compilation completes. It does not interrupt compilation in progress, and it does **not** bound **matching** time for ``parse``, ``search``, ``findall``, or similar calls. Use application-level timeouts and simple patterns for untrusted input.

**``findall``:** There is no fixed cap on how many matches can be returned within the maximum input length; consider bounding match count in your application when inputs are untrusted.

The library still enforces pattern and input size limits. You should still:

1. **Validate patterns from untrusted sources**: Check pattern complexity before use
2. **Monitor parsing performance**: Watch for unusually slow parsing operations
3. **Set your own timeouts**: If parsing takes too long in your application, implement your own timeout

.. code-block:: python

   import signal
   from formatparse import parse
   
   class TimeoutError(Exception):
       pass
   
   def timeout_handler(signum, frame):
       raise TimeoutError("Parsing timeout")
   
   def safe_parse_with_timeout(pattern, text, timeout=1.0):
       signal.signal(signal.SIGALRM, timeout_handler)
       signal.alarm(int(timeout))
       try:
           result = parse(pattern, text)
           signal.alarm(0)  # Cancel alarm
           return result
       except TimeoutError:
           signal.alarm(0)
           raise ValueError("Parsing took too long")

Resource Limits
---------------

The library enforces the following limits:

- **Pattern length**: 10,000 characters maximum
- **Input length**: 10,000,000 characters (10MB) maximum
- **Field count**: 100 fields maximum
- **Field name length**: 200 characters maximum
- **Post-compile timing check**: After each successful regex compilation, the library rejects the build if compilation took more than 200ms of wall-clock time (this is not a match-time or interruptible compile timeout; see the ReDoS section above)

If you need different limits for your use case, consider:
- Pre-validating inputs in your application
- Processing large inputs in chunks
- Using streaming parsers for very large inputs

Error Handling
--------------

Always handle errors appropriately:

.. code-block:: python

   from formatparse import parse
   
   try:
       result = parse(pattern, text)
       if result is None:
           # Pattern didn't match
           handle_no_match()
       else:
           process_result(result)
   except ValueError as e:
       # Invalid pattern or input
       log_error(e)
       handle_error()
   except Exception as e:
       # Unexpected error
       log_unexpected_error(e)
       handle_error()

Dependencies
------------

Keep your dependencies up to date:

.. code-block:: bash

   # Check for Rust vulnerabilities
   cargo audit
   
   # Check for Python vulnerabilities
   pip-audit
   
   # Update dependencies regularly
   pip install --upgrade formatparse

Performance Considerations
--------------------------

For best performance and security:

1. **Pre-compile patterns**: Use :func:`compile` for repeated patterns
2. **Cache compiled patterns**: Reuse :class:`FormatParser` instances
3. **Monitor resource usage**: Watch memory and CPU usage
4. **Set appropriate timeouts**: Don't let parsing operations hang indefinitely

.. code-block:: python

   from formatparse import compile
   
   # Compile once, use many times
   parser = compile("{name}: {age:d}")
   
   for text in many_texts:
       result = parser.parse(text)
       process_result(result)

Known Limitations
-----------------

- Very large inputs may consume significant memory
- Complex patterns may take longer to compile
- Some edge cases in pattern syntax may not be fully validated

Reporting Security Issues
-------------------------

If you discover a security vulnerability, please report it privately:
- Email: odosmatthews@gmail.com
- Do not open public GitHub issues for security vulnerabilities

See :doc:`../SECURITY` for the full security policy.

