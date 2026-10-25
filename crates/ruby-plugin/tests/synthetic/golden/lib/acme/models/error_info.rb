# frozen_string_literal: true

module Acme
  module Models
    class ErrorInfo < Acme::Internal::Type::BaseModel
      # @!attribute code
      #
      #   @return [String, nil]
      optional :code, String

      # @!attribute message
      #
      #   @return [String, nil]
      optional :message, String

      # @!method initialize(code: nil, message: nil)
      #   @param code [String]
      #   @param message [String]
    end
  end
end
