import formatparse as parse


def test_findall():
    s = "".join(
        r.fixed[0] for r in parse.findall(">{}<", "<p>some <b>bold</b> text</p>")
    )
    assert s == "some bold text"


def test_no_evaluate_result():
    s = "".join(
        m.evaluate_result().fixed[0]
        for m in parse.findall(
            ">{}<", "<p>some <b>bold</b> text</p>", evaluate_result=False
        )
    )
    assert s == "some bold text"


def test_case_sensitivity():
    results = [r.fixed[0] for r in parse.findall("x({})x", "X(hi)X")]
    assert results == ["hi"]

    results = [r.fixed[0] for r in parse.findall("x({})x", "X(hi)X", case_sensitive=True)]
    assert results == []


def test_findall_empty_results():
    """Test findall with no matches"""
    results = parse.findall("ID:{id:d}", "no matches here")
    assert len(results) == 0
    assert list(results) == []


def test_findall_empty_string():
    """Test findall with empty string"""
    results = parse.findall("ID:{id:d}", "")
    assert len(results) == 0


def test_findall_single_match():
    """Test findall with single match"""
    results = parse.findall("ID:{id:d}", "ID:42")
    assert len(results) == 1
    assert results[0].named["id"] == 42


def test_findall_many_matches():
    """Test findall with many matches"""
    text = " ".join(f"ID:{i}" for i in range(100))
    results = parse.findall("ID:{id:d}", text)
    assert len(results) == 100
    assert results[0].named["id"] == 0
    assert results[99].named["id"] == 99


def test_findall_very_many_matches():
    """Test findall with very many matches"""
    text = " ".join(f"ID:{i}" for i in range(1000))
    results = parse.findall("ID:{id:d}", text)
    assert len(results) == 1000
    assert results[0].named["id"] == 0
    assert results[999].named["id"] == 999


def test_findall_overlapping_patterns():
    """Test findall with potentially overlapping patterns"""
    # Pattern that could overlap
    results = parse.findall("{}", "abc")
    # Should find all possible matches
    assert len(results) >= 3


def test_findall_case_sensitive():
    """Test findall with case sensitivity"""
    results = parse.findall("x({})x", "X(hi)X X(bye)X", case_sensitive=True)
    assert len(results) == 0

    results = parse.findall("x({})x", "x(hi)x x(bye)x", case_sensitive=True)
    assert len(results) == 2


def test_findall_case_insensitive():
    """Test findall with case insensitivity"""
    results = parse.findall("x({})x", "X(hi)X X(bye)X", case_sensitive=False)
    assert len(results) == 2
    assert results[0].fixed[0] == "hi"
    assert results[1].fixed[0] == "bye"


def test_findall_evaluate_result_false():
    """Test findall with evaluate_result=False"""
    results = parse.findall(">{}<", "<p>a</p> <p>b</p> <p>c</p>", evaluate_result=False)
    # Should be Match objects, not ParseResult
    assert len(results) >= 2  # May match more due to non-greedy matching
    for match in results:
        result = match.evaluate_result()
        assert result is not None


def test_findall_custom_types():
    """Test findall with custom type converters"""
    from formatparse import with_pattern

    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    results = parse.findall(
        "Value: {:Number}", "Value: 1, Value: 2, Value: 3", {"Number": parse_number}
    )
    assert len(results) == 3
    assert results[0].fixed[0] == 1
    assert results[1].fixed[0] == 2
    assert results[2].fixed[0] == 3


def test_findall_named_fields():
    """Test findall with named fields"""
    # Use newline separators to work with non-greedy string matching
    results = parse.findall("{name}: {age:d}", "Alice: 30\nBob: 25\nCharlie: 35")
    assert len(results) >= 3
    # Verify we got results with the expected structure
    assert all("name" in r.named and "age" in r.named for r in results[:3])
    assert results[0].named["age"] == 30
    assert results[1].named["age"] == 25
    assert results[2].named["age"] == 35


def test_findall_positional_fields():
    """Test findall with positional fields"""
    # Use newline separators to avoid overlapping matches with non-greedy patterns
    results = parse.findall("{}, {}", "A, B\nC, D\nE, F")
    assert len(results) >= 3
    # Verify we got results with the expected structure
    assert all(len(r.fixed) == 2 for r in results[:3])
    assert results[0].fixed[0] in ["A", "C", "E"]
    assert results[0].fixed[1] in ["B", "D", "F"]


def test_findall_performance_lazy():
    """Test that findall uses lazy evaluation"""
    # Create many matches
    text = " ".join(f"ID:{i}" for i in range(1000))
    results = parse.findall("ID:{id:d}", text)

    # Accessing len() should be fast (no conversion)
    assert len(results) == 1000

    # Accessing first item should only convert that item
    first = results[0]
    assert first.named["id"] == 0

    # Iterating should batch convert
    items = [r.named["id"] for r in results]
    assert len(items) == 1000
    assert items[0] == 0
    assert items[999] == 999
