# frozen_string_literal: true

# Standard libraries.
# rubocop:disable Lint/RedundantRequireStatement
require "English"
require "base64"
require "cgi"
require "date"
require "erb"
require "etc"
require "json"
require "net/http"
require "openssl"
require "pathname"
require "rbconfig"
require "securerandom"
require "set"
require "stringio"
require "time"
require "uri"
# rubocop:enable Lint/RedundantRequireStatement

# We already ship the preferred sorbet manifests in the package itself.
# `tapioca` currently does not offer us a way to opt out of unnecessary compilation.
if Object.const_defined?(:Tapioca) &&
   caller.chain([$PROGRAM_NAME]).chain(ARGV).any?(/tapioca/) &&
   ARGV.none?(/dsl/)
  return
end


# Gems.
require "connection_pool"

# Package files.
require_relative "acme/version"
require_relative "acme/internal/util"
require_relative "acme/internal/type/converter"
require_relative "acme/internal/type/unknown"
require_relative "acme/internal/type/boolean"
require_relative "acme/internal/type/file_input"
require_relative "acme/internal/type/enum"
require_relative "acme/internal/type/union"
require_relative "acme/internal/type/array_of"
require_relative "acme/internal/type/hash_of"
require_relative "acme/internal/type/base_model"
require_relative "acme/internal/type/base_page"
require_relative "acme/internal/type/request_parameters"
require_relative "acme/internal"
require_relative "acme/request_options"
require_relative "acme/file_part"
require_relative "acme/errors"
require_relative "acme/internal/transport/base_client"
require_relative "acme/internal/transport/pooled_net_requester"
require_relative "acme/client"
require_relative "acme/models/error_info"
require_relative "acme/models/labels"
require_relative "acme/models/widget"
require_relative "acme/models/widget_create_params"
require_relative "acme/models/widget_delete_params"
require_relative "acme/models/widget_list_params"
require_relative "acme/models/widget_list_response"
require_relative "acme/models/widget_retrieve_params"
require_relative "acme/models/widgets/part_retrieve_params"
require_relative "acme/models/widgets/part_retrieve_response"
require_relative "acme/models"
require_relative "acme/resources/widgets"
require_relative "acme/resources/widgets/parts"
