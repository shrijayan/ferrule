# frozen_string_literal: true

module Acme
  module Models
    # @see Acme::Resources::Widgets#list
    class WidgetListResponse < Acme::Internal::Type::BaseModel
      # @!attribute data
      #
      #   @return [Array<Acme::Models::Widget>]
      required :data, -> { Acme::Internal::Type::ArrayOf[Acme::Widget] }

      # @!attribute next_cursor
      #
      #   @return [String, nil]
      optional :next_cursor, String, nil?: true

      # @!method initialize(data:, next_cursor: nil)
      #   @param data [Array<Acme::Models::Widget>]
      #   @param next_cursor [String, nil]
    end
  end
end
