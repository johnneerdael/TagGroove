// Independent C ABI adapter. Boost.Regex is vendored under BSL-1.0.
#include <boost/regex.hpp>
#include <cstdint>
#include <limits>
#include <locale>
#include <string>
#include <vector>
#include <algorithm>
#include <mutex>

static thread_local std::string last_error;

// Boost.Regex's locale-dependent traits are not safe to set up from several
// threads at once (crashes seen when expressions were compiled in parallel),
// so compilation is serialised. Matching a compiled expression is const and
// stays concurrent.
static std::mutex& compile_mutex() {
    static std::mutex mutex;
    return mutex;
}
static const std::locale& user_locale() {
    static const std::locale locale("");
    return locale;
}

// Preserve Windows UTF-16 code units on platforms with 32-bit wchar_t.
static std::wstring wide(const uint16_t* text, size_t count) {
    std::wstring result;
    result.reserve(count);
    for (size_t i = 0; i < count; ++i) result.push_back(static_cast<wchar_t>(text[i]));
    return result;
}

extern "C" {
const char* ot_regex_error() noexcept { return last_error.c_str(); }

void* ot_regex_new(const uint16_t* text, size_t count, bool sensitive) noexcept {
    try {
        std::lock_guard<std::mutex> lock(compile_mutex());
        auto expression = new boost::wregex();
        try {
            expression->imbue(user_locale());
            const auto flags = boost::regex::perl | (sensitive ? boost::regex::normal : boost::regex::icase);
            expression->assign(wide(text, count), flags);
            return expression;
        } catch (...) { delete expression; throw; }
    } catch (const std::exception& error) { last_error = error.what(); }
    catch (...) { last_error = "Unknown regular expression compilation error"; }
    return nullptr;
}

void ot_regex_free(void* expression) noexcept { delete static_cast<boost::wregex*>(expression); }
size_t ot_regex_groups(const void* expression) noexcept {
    return static_cast<const boost::wregex*>(expression)->mark_count() + 1;
}

int ot_regex_captures(const void* expression, const uint16_t* text, size_t count,
                      size_t* positions, size_t* lengths, size_t capacity) noexcept {
    try {
        const auto input = wide(text, count);
        boost::wsmatch match;
        if (!boost::regex_search(input, match, *static_cast<const boost::wregex*>(expression))) return 0;
        for (size_t i = 0; i < capacity && i < match.size(); ++i) {
            positions[i] = match[i].matched ? static_cast<size_t>(match.position(i)) : std::numeric_limits<size_t>::max();
            lengths[i] = match[i].matched ? static_cast<size_t>(match.length(i)) : 0;
        }
        return 1;
    } catch (const std::exception& error) { last_error = error.what(); }
    catch (...) { last_error = "Unknown regular expression matching error"; }
    return -1;
}

uint16_t* ot_regex_replace(const void* expression, const uint16_t* text, size_t count,
                          const uint16_t* replacement, size_t replacement_count, size_t* output_count) noexcept {
    try {
        const auto result = boost::regex_replace(wide(text, count), *static_cast<const boost::wregex*>(expression),
                                                wide(replacement, replacement_count), boost::format_default);
        auto output = new uint16_t[result.size()];
        for (size_t i = 0; i < result.size(); ++i) output[i] = static_cast<uint16_t>(result[i]);
        *output_count = result.size();
        return output;
    } catch (const std::exception& error) { last_error = error.what(); }
    catch (...) { last_error = "Unknown regular expression replacement error"; }
    return nullptr;
}
void ot_regex_output_free(uint16_t* text) noexcept { delete[] text; }

// Every non-overlapping match as (start, length) pairs in UTF-16 units.
size_t* ot_regex_find_all(const void* expression, const uint16_t* text, size_t count, size_t* found) noexcept {
    try {
        const auto input = wide(text, count);
        std::vector<size_t> spans;
        const auto& pattern = *static_cast<const boost::wregex*>(expression);
        for (boost::wsregex_iterator it(input.begin(), input.end(), pattern), end; it != end; ++it) {
            spans.push_back(static_cast<size_t>((*it)[0].first - input.begin()));
            spans.push_back(static_cast<size_t>((*it)[0].length()));
        }
        auto output = new size_t[spans.size() + 1];
        std::copy(spans.begin(), spans.end(), output);
        *found = spans.size() / 2;
        return output;
    } catch (const std::exception& error) { last_error = error.what(); }
    catch (...) { last_error = "Unknown regular expression matching error"; }
    return nullptr;
}
void ot_regex_spans_free(size_t* spans) noexcept { delete[] spans; }
}
